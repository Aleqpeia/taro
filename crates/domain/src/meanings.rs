//! Authored card interpretations, loaded from the bundled `data/meanings.ron`.
//!
//! This is the always-on, offline interpretation layer. The optional Claude
//! "deeper reading" (Phase 5) composes a narrative on top of these.

use std::collections::HashMap;

use serde::Deserialize;

use crate::card::{Card, Orientation};

/// Upright and reversed meanings for a single card.
#[derive(Debug, Clone, Deserialize)]
pub struct CardMeaning {
    pub upright: String,
    pub reversed: String,
}

impl CardMeaning {
    pub fn for_orientation(&self, orientation: Orientation) -> &str {
        match orientation {
            Orientation::Upright => &self.upright,
            Orientation::Reversed => &self.reversed,
        }
    }
}

/// The full meanings table, keyed by [`Card::id`].
#[derive(Debug, Clone, Deserialize)]
pub struct Meanings {
    pub cards: HashMap<String, CardMeaning>,
}

const EMBEDDED: &str = include_str!("../data/meanings.ron");

impl Meanings {
    /// Load the meanings bundled into the binary. Panics only if the bundled
    /// data is malformed — which the test suite guards against.
    pub fn embedded() -> Self {
        ron::from_str(EMBEDDED).expect("bundled data/meanings.ron must parse")
    }

    pub fn get(&self, card: Card) -> Option<&CardMeaning> {
        self.cards.get(&card.id())
    }

    pub fn text(&self, card: Card, orientation: Orientation) -> Option<&str> {
        self.get(card).map(|m| m.for_orientation(orientation))
    }
}
