//! Composing a [`Reading`]: pairing each spread position with its drawn card and
//! the authored meaning for that card's orientation.

use crate::card::Orientation;
use crate::deck::DrawnCard;
use crate::meanings::Meanings;
use crate::spread::Spread;

/// One line of a reading: a position, the card that landed there, and its meaning.
#[derive(Debug, Clone)]
pub struct ReadingEntry {
    pub position_index: usize,
    pub position_name: String,
    pub position_description: String,
    pub drawn: DrawnCard,
    pub card_name: String,
    pub meaning: String,
}

/// A complete reading for one spread.
#[derive(Debug, Clone)]
pub struct Reading {
    pub spread: String,
    pub entries: Vec<ReadingEntry>,
}

/// Build a reading by zipping the spread's positions with the drawn cards.
///
/// Pairs are formed in order, so `drawn` should contain at least
/// `spread.card_count()` cards; extra cards are ignored.
pub fn build_reading(spread: &dyn Spread, drawn: &[DrawnCard], meanings: &Meanings) -> Reading {
    let entries = spread
        .positions()
        .iter()
        .zip(drawn.iter())
        .map(|(pos, &dc)| {
            let meaning = meanings
                .text(dc.card, dc.orientation)
                .unwrap_or("(no meaning available)")
                .to_string();
            ReadingEntry {
                position_index: pos.index,
                position_name: pos.name.to_string(),
                position_description: pos.description.to_string(),
                drawn: dc,
                card_name: dc.card.display_name(),
                meaning,
            }
        })
        .collect();

    Reading {
        spread: spread.name().to_string(),
        entries,
    }
}

fn orient_word(o: Orientation) -> &'static str {
    match o {
        Orientation::Upright => "upright",
        Orientation::Reversed => "reversed",
    }
}

/// One woven sentence for an entry, framed by its Celtic Cross position.
fn entry_sentence(e: &ReadingEntry) -> String {
    let card = &e.card_name;
    let o = orient_word(e.drawn.orientation);
    let m = e.meaning.trim();
    match e.position_index {
        1 => format!("At the heart of the matter lies {card}, {o}: {m}"),
        2 => format!("Crossing it comes {card}, {o} — the challenge of the spread: {m}"),
        3 => format!("Beneath, as the foundation, {card} rests {o}: {m}"),
        4 => format!("Behind you, already passing, {card} shows {o}: {m}"),
        5 => format!("Above, as the crown of what may be, {card} stands {o}: {m}"),
        6 => format!("And just ahead, {card} approaches {o}: {m}"),
        7 => format!("Within yourself, {card} sits {o}: {m}"),
        8 => format!("Around you, the world turns up {card}, {o}: {m}"),
        9 => format!("In your hopes and fears, {card} stirs {o}: {m}"),
        10 => format!("And the path leads, at last, to {card}, {o}: {m}"),
        _ => format!("{} — {card}, {o}: {m}", e.position_name),
    }
}

/// Weave a whole [`Reading`] into one flowing narrative across all positions —
/// the offline, deterministic counterpart to the optional Claude "deeper
/// reading" (Phase 5). The cross (positions 1–6) and the staff (7–10) are read
/// as two movements, closing on the Outcome. Pass the querent's `question` to
/// frame the opening; `None` (or blank) gives a generic frame.
pub fn compose_reading(reading: &Reading, question: Option<&str>) -> String {
    let mut out = String::new();

    match question.map(str::trim).filter(|q| !q.is_empty()) {
        Some(q) => {
            out.push_str(&format!("You asked: «{q}»\n\n"));
            out.push_str(
                "The Celtic Cross answers in two movements — the cross at the centre, \
                 and the staff that runs beside it.\n\n",
            );
        }
        None => out.push_str(
            "The Celtic Cross falls in two movements — the cross at the centre, \
             and the staff that runs beside it.\n\n",
        ),
    }

    let cross: Vec<String> = reading
        .entries
        .iter()
        .filter(|e| e.position_index <= 6)
        .map(entry_sentence)
        .collect();
    let staff: Vec<String> = reading
        .entries
        .iter()
        .filter(|e| e.position_index >= 7)
        .map(entry_sentence)
        .collect();

    if !cross.is_empty() {
        out.push_str("The Cross\n");
        out.push_str(&cross.join(" "));
        out.push_str("\n\n");
    }
    if !staff.is_empty() {
        out.push_str("The Staff\n");
        out.push_str(&staff.join(" "));
        out.push_str("\n\n");
    }

    if let Some(outcome) = reading.entries.iter().find(|e| e.position_index == 10) {
        out.push_str(&format!(
            "In the end the spread bends toward {} at the outcome. \
             Read it as a mirror for reflection, not a fixed decree.",
            outcome.card_name
        ));
    }

    out
}
