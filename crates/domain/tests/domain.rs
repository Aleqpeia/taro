use std::collections::HashSet;

use rand::rngs::StdRng;
use rand::SeedableRng;

use taro_domain::{build_reading, compose_reading, CelticCross, Deck, Meanings, Spread};

#[test]
fn deck_has_78_unique_cards() {
    let deck = Deck::full();
    assert_eq!(deck.len(), 78);
    let ids: HashSet<String> = deck.cards().iter().map(|c| c.id()).collect();
    assert_eq!(ids.len(), 78, "card ids must be unique");
}

#[test]
fn every_card_resolves_to_a_meaning() {
    let meanings = Meanings::embedded();
    for card in Deck::full().cards() {
        let m = meanings
            .get(*card)
            .unwrap_or_else(|| panic!("missing meaning for {} ({})", card.display_name(), card.id()));
        assert!(!m.upright.trim().is_empty(), "empty upright for {}", card.id());
        assert!(!m.reversed.trim().is_empty(), "empty reversed for {}", card.id());
    }
}

#[test]
fn meanings_table_has_no_orphan_keys() {
    // Every authored key should correspond to a real card in the deck.
    let valid: HashSet<String> = Deck::full().cards().iter().map(|c| c.id()).collect();
    let meanings = Meanings::embedded();
    for key in meanings.cards.keys() {
        assert!(valid.contains(key), "meanings.ron has orphan key: {key}");
    }
    assert_eq!(meanings.cards.len(), 78);
}

#[test]
fn celtic_cross_has_ten_positions_with_one_crossing_card() {
    assert_eq!(CelticCross.card_count(), 10);
    assert_eq!(CelticCross.positions().len(), 10);

    let indices: Vec<usize> = CelticCross.positions().iter().map(|p| p.index).collect();
    assert_eq!(indices, (1..=10).collect::<Vec<_>>());

    let rotated = CelticCross
        .positions()
        .iter()
        .filter(|p| p.slot.rotated)
        .count();
    assert_eq!(rotated, 1, "exactly one card (the Challenge) is laid sideways");
}

#[test]
fn draw_is_non_repeating_and_correctly_sized() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut deck = Deck::full();
    deck.shuffle(&mut rng);

    let drawn = deck.draw(&mut rng, 10);
    assert_eq!(drawn.len(), 10);
    assert_eq!(deck.len(), 68, "drawn cards leave the deck");

    let ids: HashSet<String> = drawn.iter().map(|d| d.card.id()).collect();
    assert_eq!(ids.len(), 10, "no card is drawn twice");
}

#[test]
fn draw_caps_at_deck_size() {
    let mut rng = StdRng::seed_from_u64(1);
    let mut deck = Deck::full();
    let drawn = deck.draw(&mut rng, 1000);
    assert_eq!(drawn.len(), 78);
    assert!(deck.is_empty());
}

#[test]
fn shuffle_is_deterministic_for_a_given_seed() {
    let order = |seed| {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut deck = Deck::full();
        deck.shuffle(&mut rng);
        deck.cards().iter().map(|c| c.id()).collect::<Vec<_>>()
    };
    assert_eq!(order(99), order(99), "same seed -> same order");
    assert_ne!(order(1), order(2), "different seeds -> different order");
}

#[test]
fn reading_covers_every_position_with_a_real_meaning() {
    let mut rng = StdRng::seed_from_u64(7);
    let mut deck = Deck::full();
    deck.shuffle(&mut rng);
    let drawn = deck.draw(&mut rng, CelticCross.card_count());

    let meanings = Meanings::embedded();
    let reading = build_reading(&CelticCross, &drawn, &meanings);

    assert_eq!(reading.spread, "Celtic Cross");
    assert_eq!(reading.entries.len(), 10);
    for e in &reading.entries {
        assert!(!e.card_name.is_empty());
        assert!(!e.meaning.contains("no meaning available"), "missing meaning at position {}", e.position_index);
    }
}

#[test]
fn composed_reading_weaves_every_card_meaning_and_the_question() {
    let mut rng = StdRng::seed_from_u64(7);
    let mut deck = Deck::full();
    deck.shuffle(&mut rng);
    let drawn = deck.draw(&mut rng, CelticCross.card_count());
    let meanings = Meanings::embedded();
    let reading = build_reading(&CelticCross, &drawn, &meanings);

    let q = "Will the move work out?";
    let text = compose_reading(&reading, Some(q));

    assert!(text.contains(q), "the question should appear in the narrative");
    assert!(text.contains("The Cross") && text.contains("The Staff"), "both movements present");
    for e in &reading.entries {
        assert!(text.contains(&e.card_name), "card {} missing from narrative", e.card_name);
        assert!(text.contains(e.meaning.trim()), "meaning for {} missing", e.card_name);
    }
}

#[test]
fn composed_reading_without_question_is_nonempty_and_omits_the_ask() {
    let mut rng = StdRng::seed_from_u64(3);
    let mut deck = Deck::full();
    deck.shuffle(&mut rng);
    let drawn = deck.draw(&mut rng, CelticCross.card_count());
    let meanings = Meanings::embedded();
    let reading = build_reading(&CelticCross, &drawn, &meanings);

    // None and blank are treated the same: a generic frame, no "You asked".
    for q in [None, Some("   ")] {
        let text = compose_reading(&reading, q);
        assert!(!text.trim().is_empty());
        assert!(!text.contains("You asked"), "no question frame for {q:?}");
    }
}
