//! Phase 5: the optional AI "deeper reading" — a streamed narrative woven
//! from the drawn spread, layered on top of the offline `compose_reading`.
//!
//! Three providers are supported: Anthropic (the Messages REST API, called
//! directly — there is no official Rust SDK), plus OpenRouter and Groq, which
//! both speak the OpenAI-compatible `chat/completions` protocol and share one
//! code path. Instead of the tokio → Bevy bridge sketched in PLAN.md, the
//! request runs on a plain `std::thread` with `reqwest::blocking` (whose
//! response implements `Read`, so the SSE stream parses line by line) and
//! feeds text deltas over an `mpsc` channel that [`poll_deep_reading`] drains
//! each frame. Same effect, no async runtime to plumb through the ECS.
//!
//! Fully optional: enabled when any provider has a key (env var or keyring,
//! stored via `taro-app --set-api-key [PROVIDER]`). `TARO_AI_PROVIDER` picks
//! the provider explicitly; otherwise the first one with a key wins. `D`
//! inside the full-reading overlay requests a reading; `TARO_AI_MODEL`
//! overrides the model; the env hook `TARO_DEEP_READING_AT=secs` fires one
//! request for manual/harness testing.

use std::io::{BufRead, BufReader};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Mutex;
use std::time::Duration;

use bevy::prelude::*;

use taro_domain::Reading;

use crate::question::QuestionInput;
use crate::reading_view::{spread_settled, ShowFullReading};
use crate::{DealInfo, ReadingData};

/// Service name of the stored secrets in the OS keyring.
pub const KEYRING_SERVICE: &str = "taro";

/// Which backend serves the deeper reading. Anthropic keeps its native
/// Messages API; OpenRouter (one key, many hosted models) and Groq (fast
/// inference with a free tier) both speak the OpenAI-compatible
/// `chat/completions` protocol, so they share a request/parse path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Anthropic,
    OpenRouter,
    Groq,
}

impl Provider {
    /// Detection preference order (also the `--set-api-key` provider names).
    pub const ALL: [Provider; 3] = [Provider::Anthropic, Provider::OpenRouter, Provider::Groq];

    pub fn from_name(name: &str) -> Option<Provider> {
        match name.trim().to_ascii_lowercase().as_str() {
            "anthropic" | "claude" => Some(Provider::Anthropic),
            "openrouter" => Some(Provider::OpenRouter),
            "groq" => Some(Provider::Groq),
            _ => None,
        }
    }

    /// How the provider is called out in the UI hint line.
    pub fn label(self) -> &'static str {
        match self {
            Provider::Anthropic => "Claude",
            Provider::OpenRouter => "OpenRouter",
            Provider::Groq => "Groq",
        }
    }

    pub fn key_env(self) -> &'static str {
        match self {
            Provider::Anthropic => "ANTHROPIC_API_KEY",
            Provider::OpenRouter => "OPENROUTER_API_KEY",
            Provider::Groq => "GROQ_API_KEY",
        }
    }

    /// Username of this provider's secret in the OS keyring.
    pub fn keyring_user(self) -> &'static str {
        match self {
            Provider::Anthropic => "anthropic-api-key",
            Provider::OpenRouter => "openrouter-api-key",
            Provider::Groq => "groq-api-key",
        }
    }

    fn api_url(self) -> &'static str {
        match self {
            Provider::Anthropic => "https://api.anthropic.com/v1/messages",
            Provider::OpenRouter => "https://openrouter.ai/api/v1/chat/completions",
            Provider::Groq => "https://api.groq.com/openai/v1/chat/completions",
        }
    }

    fn default_model(self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-opus-4-8",
            // Let OpenRouter route to a suitable model; TARO_AI_MODEL can pin
            // a specific one (e.g. a reasoning model) instead.
            Provider::OpenRouter => "openrouter/auto",
            Provider::Groq => "llama-3.3-70b-versatile",
        }
    }

    /// The active provider: `TARO_AI_PROVIDER` wins when set (and names a
    /// known provider), otherwise the first provider with a key. `None`
    /// disables the AI path.
    pub fn detect() -> Option<Provider> {
        if let Ok(name) = std::env::var("TARO_AI_PROVIDER") {
            if !name.trim().is_empty() {
                return Provider::from_name(&name).filter(|p| api_key(*p).is_some());
            }
        }
        Provider::ALL.into_iter().find(|p| api_key(*p).is_some())
    }
}

/// A provider's API key: its env var wins, then the OS keyring (stored via
/// `taro-app --set-api-key [PROVIDER]`).
pub fn api_key(provider: Provider) -> Option<String> {
    if let Ok(k) = std::env::var(provider.key_env()) {
        let k = k.trim();
        if !k.is_empty() {
            return Some(k.to_string());
        }
    }
    keyring::Entry::new(KEYRING_SERVICE, provider.keyring_user())
        .ok()?
        .get_password()
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
}

const SYSTEM_PROMPT: &str = "You are a thoughtful Tarot de Marseille reader. Interpret the \
    spread holistically: connect the cards across positions rather than reciting them one by \
    one, ground yourself in the traditional meanings provided, and speak directly to the \
    querent in a warm, unhurried voice. Present the reading as a mirror for reflection, not a \
    prediction of fixed fact. Answer in roughly 300–380 words of plain flowing prose — no \
    headings, no lists, no markdown.";

/// One parsed event out of the SSE stream (or the worker thread's verdict).
pub enum AiEvent {
    Delta(String),
    Done,
    Error(String),
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepState {
    #[default]
    Idle,
    Streaming,
    Done,
    Error,
}

/// The deeper reading's lifecycle + streamed text. The receiver end of the
/// worker channel lives here (in a `Mutex` because `mpsc::Receiver` is `!Sync`
/// and Bevy resources must be `Send + Sync`).
#[derive(Resource)]
pub struct DeepReading {
    pub state: DeepState,
    pub text: String,
    pub error: String,
    /// The provider that will serve the reading; `None` disables the AI path
    /// (no key found for any provider).
    pub provider: Option<Provider>,
    rx: Option<Mutex<Receiver<AiEvent>>>,
}

impl DeepReading {
    pub fn detect() -> Self {
        Self {
            state: DeepState::Idle,
            text: String::new(),
            error: String::new(),
            provider: Provider::detect(),
            rx: None,
        }
    }

    pub fn available(&self) -> bool {
        self.provider.is_some()
    }

    /// Forget any in-flight or finished reading (used on redeals; dropping the
    /// receiver makes the worker thread's next send fail, so it just exits).
    pub fn reset(&mut self) {
        self.state = DeepState::Idle;
        self.text.clear();
        self.error.clear();
        self.rx = None;
    }
}

/// The user prompt: the whole drawn spread plus the querent's question.
pub fn build_prompt(reading: &Reading, question: Option<&str>) -> String {
    let mut p = format!("Spread: {}.\n", reading.spread);
    if let Some(q) = question.map(str::trim).filter(|q| !q.is_empty()) {
        p.push_str(&format!("The querent asks: {q}\n"));
    }
    p.push_str("Cards drawn:\n");
    for e in &reading.entries {
        p.push_str(&format!(
            "{}. {} — {} ({}). Traditional meaning: {}\n",
            e.position_index,
            e.position_name,
            e.card_name,
            e.drawn.orientation.label(),
            e.meaning,
        ));
    }
    p.push_str("\nWeave these into one coherent reading.");
    p
}

/// Parse one OpenAI-style SSE line (OpenRouter, Groq) into an event. Lines
/// that aren't `data:` payloads, role-only/empty deltas, and reasoning deltas
/// (reasoning models stream those separately) yield `None`.
pub fn parse_openai_sse_line(line: &str) -> Option<AiEvent> {
    let data = line.strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return Some(AiEvent::Done);
    }
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    if let Some(msg) = v["error"]["message"].as_str() {
        return Some(AiEvent::Error(msg.to_string()));
    }
    v["choices"][0]["delta"]["content"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| AiEvent::Delta(s.to_string()))
}

/// Parse one Anthropic SSE line into an event. Lines that aren't `data:`
/// payloads, and stream events we don't act on (message_start,
/// content_block_start, thinking deltas, pings...), yield `None`.
pub fn parse_sse_line(line: &str) -> Option<AiEvent> {
    let data = line.strip_prefix("data:")?.trim();
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    match v["type"].as_str()? {
        "content_block_delta" if v["delta"]["type"] == "text_delta" => {
            v["delta"]["text"].as_str().map(|s| AiEvent::Delta(s.to_string()))
        }
        "message_stop" => Some(AiEvent::Done),
        "error" => Some(AiEvent::Error(
            v["error"]["message"].as_str().unwrap_or("stream error").to_string(),
        )),
        _ => None,
    }
}

/// Spawn the worker thread; events arrive on the returned channel.
fn spawn_request(prompt: String) -> Receiver<AiEvent> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        if let Err(e) = run_request(&prompt, &tx) {
            let _ = tx.send(AiEvent::Error(e));
        }
    });
    rx
}

/// Blocking POST + SSE read loop, entirely on the worker thread.
fn run_request(prompt: &str, tx: &Sender<AiEvent>) -> Result<(), String> {
    let provider = Provider::detect().ok_or_else(|| "no API key in env or keyring".to_string())?;
    let key = api_key(provider)
        .ok_or_else(|| format!("no {} API key in env or keyring", provider.label()))?;
    let model =
        std::env::var("TARO_AI_MODEL").unwrap_or_else(|_| provider.default_model().to_string());

    let body = match provider {
        Provider::Anthropic => serde_json::json!({
            "model": model,
            "max_tokens": 4000,
            "stream": true,
            "thinking": {"type": "adaptive"},
            "system": SYSTEM_PROMPT,
            "messages": [{"role": "user", "content": prompt}],
        }),
        // OpenAI-compatible: the system prompt rides as the first message.
        Provider::OpenRouter | Provider::Groq => serde_json::json!({
            "model": model,
            "max_tokens": 4000,
            "stream": true,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": prompt},
            ],
        }),
    };

    let client = reqwest::blocking::Client::builder()
        // No whole-request deadline: the stream stays open while the model types.
        .timeout(None)
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let req = client
        .post(provider.api_url())
        .header("content-type", "application/json")
        .body(body.to_string());
    let req = match provider {
        Provider::Anthropic => req
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01"),
        Provider::OpenRouter | Provider::Groq => {
            req.header("authorization", format!("Bearer {key}"))
        }
    };
    let resp = req.send().map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        let msg = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v["error"]["message"].as_str().map(String::from))
            .unwrap_or(text);
        return Err(format!("{status}: {msg}"));
    }

    let parse: fn(&str) -> Option<AiEvent> = match provider {
        Provider::Anthropic => parse_sse_line,
        Provider::OpenRouter | Provider::Groq => parse_openai_sse_line,
    };
    for line in BufReader::new(resp).lines() {
        let line = line.map_err(|e| e.to_string())?;
        match parse(&line) {
            Some(AiEvent::Done) => {
                let _ = tx.send(AiEvent::Done);
                return Ok(());
            }
            Some(ev) => {
                // A failed send means the UI dropped us (redeal) — just stop.
                if tx.send(ev).is_err() {
                    return Ok(());
                }
            }
            None => {}
        }
    }
    let _ = tx.send(AiEvent::Done);
    Ok(())
}

/// `D` inside the full-reading overlay asks the AI provider for a deeper
/// reading, once the spread has settled. The env `TARO_DEEP_READING_AT=secs`
/// fires one request without a keypress.
#[allow(clippy::too_many_arguments)]
pub fn deep_reading_input(
    keys: Res<ButtonInput<KeyCode>>,
    q: Res<QuestionInput>,
    show: Res<ShowFullReading>,
    reading: Option<Res<ReadingData>>,
    deal_info: Option<Res<DealInfo>>,
    time: Res<Time>,
    mut at: Local<Option<f32>>,
    mut init: Local<bool>,
    mut dr: ResMut<DeepReading>,
) {
    if !*init {
        *init = true;
        *at = std::env::var("TARO_DEEP_READING_AT").ok().and_then(|s| s.parse().ok());
    }
    let env_fire = at.is_some_and(|t| time.elapsed_secs() >= t);
    if env_fire {
        *at = None;
    } else if q.editing || !show.0 || !keys.just_pressed(KeyCode::KeyD) {
        return;
    }

    if !dr.available() || dr.state == DeepState::Streaming {
        return;
    }
    let Some(reading) = reading else { return };
    if !spread_settled(deal_info.as_deref(), time.elapsed_secs()) {
        return;
    }

    let question = (!q.text.trim().is_empty()).then(|| q.text.trim().to_string());
    let prompt = build_prompt(&reading.0, question.as_deref());
    dr.text.clear();
    dr.error.clear();
    dr.state = DeepState::Streaming;
    dr.rx = Some(Mutex::new(spawn_request(prompt)));
}

/// Drain the worker channel into the resource; the overlay renders `text`.
pub fn poll_deep_reading(mut dr: ResMut<DeepReading>) {
    let dr = &mut *dr;
    let Some(rx) = &dr.rx else { return };
    let mut terminal = false;
    {
        let rx = rx.lock().unwrap();
        loop {
            match rx.try_recv() {
                Ok(AiEvent::Delta(s)) => dr.text.push_str(&s),
                Ok(AiEvent::Done) => {
                    dr.state = DeepState::Done;
                    terminal = true;
                }
                Ok(AiEvent::Error(e)) => {
                    dr.state = DeepState::Error;
                    dr.error = e;
                    terminal = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if dr.state == DeepState::Streaming {
                        dr.state = DeepState::Error;
                        dr.error = "the stream ended unexpectedly".to_string();
                    }
                    terminal = true;
                    break;
                }
            }
        }
    }
    if terminal {
        dr.rx = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;
    use taro_domain::{build_reading, CelticCross, Deck, Meanings, Spread};

    fn a_reading() -> Reading {
        let mut deck = Deck::full();
        deck.shuffle(&mut thread_rng());
        let spread = CelticCross;
        let drawn = deck.draw(&mut thread_rng(), spread.card_count());
        build_reading(&spread, &drawn, &Meanings::embedded())
    }

    #[test]
    fn prompt_names_every_position_and_the_question() {
        let reading = a_reading();
        let p = build_prompt(&reading, Some("Will the garden grow?"));
        assert!(p.contains("Will the garden grow?"));
        for e in &reading.entries {
            assert!(p.contains(&e.card_name), "missing {}", e.card_name);
            assert!(p.contains(&e.position_name), "missing {}", e.position_name);
        }
    }

    #[test]
    fn blank_question_is_omitted() {
        let p = build_prompt(&a_reading(), Some("   "));
        assert!(!p.contains("The querent asks"));
    }

    #[test]
    fn sse_text_deltas_and_stop_parse() {
        let delta = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"The cards"}}"#;
        match parse_sse_line(delta) {
            Some(AiEvent::Delta(s)) => assert_eq!(s, "The cards"),
            _ => panic!("expected a text delta"),
        }
        assert!(matches!(parse_sse_line(r#"data: {"type":"message_stop"}"#), Some(AiEvent::Done)));
    }

    #[test]
    fn sse_noise_is_ignored() {
        assert!(parse_sse_line("event: content_block_delta").is_none());
        assert!(parse_sse_line("").is_none());
        // Thinking deltas (adaptive thinking) must not leak into the reading.
        let thinking = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"hmm"}}"#;
        assert!(parse_sse_line(thinking).is_none());
    }

    #[test]
    fn sse_error_events_surface_the_message() {
        let err = r#"data: {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
        match parse_sse_line(err) {
            Some(AiEvent::Error(m)) => assert_eq!(m, "Overloaded"),
            _ => panic!("expected an error event"),
        }
    }

    #[test]
    fn openai_sse_content_deltas_and_done_parse() {
        let delta = r#"data: {"id":"x","choices":[{"index":0,"delta":{"content":"The cards"}}]}"#;
        match parse_openai_sse_line(delta) {
            Some(AiEvent::Delta(s)) => assert_eq!(s, "The cards"),
            _ => panic!("expected a content delta"),
        }
        assert!(matches!(parse_openai_sse_line("data: [DONE]"), Some(AiEvent::Done)));
    }

    #[test]
    fn openai_sse_noise_is_ignored() {
        assert!(parse_openai_sse_line(": OPENROUTER PROCESSING").is_none());
        assert!(parse_openai_sse_line("").is_none());
        // Role-only first chunk and empty-content keepalives carry no text.
        let role = r#"data: {"choices":[{"delta":{"role":"assistant","content":""}}]}"#;
        assert!(parse_openai_sse_line(role).is_none());
        // Reasoning-model deltas must not leak into the reading.
        let reasoning = r#"data: {"choices":[{"delta":{"reasoning":"hmm"}}]}"#;
        assert!(parse_openai_sse_line(reasoning).is_none());
    }

    #[test]
    fn openai_sse_error_events_surface_the_message() {
        let err = r#"data: {"error":{"message":"Rate limit exceeded","code":429}}"#;
        match parse_openai_sse_line(err) {
            Some(AiEvent::Error(m)) => assert_eq!(m, "Rate limit exceeded"),
            _ => panic!("expected an error event"),
        }
    }

    #[test]
    fn provider_names_resolve() {
        assert_eq!(Provider::from_name("OpenRouter"), Some(Provider::OpenRouter));
        assert_eq!(Provider::from_name("groq"), Some(Provider::Groq));
        assert_eq!(Provider::from_name("claude"), Some(Provider::Anthropic));
        assert_eq!(Provider::from_name("mystery"), None);
    }
}
