//! The 78-card deck plus shuffle and draw operations.

use rand::seq::SliceRandom;
use rand::Rng;

use crate::card::{Card, Orientation, Rank, Suit};

/// A card as it appears in a reading: the card plus the orientation it was
/// drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnCard {
    pub card: Card,
    pub orientation: Orientation,
}

/// An ordered stack of cards. Drawing takes from the top (the end of the vec).
#[derive(Debug, Clone)]
pub struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    /// A complete, ordered 78-card deck: 22 Major Arcana followed by the four
    /// suits of Minor Arcana, each Ace → Roi.
    pub fn full() -> Self {
        let mut cards = Vec::with_capacity(78);
        for number in 0..22u8 {
            cards.push(Card::Major { number });
        }
        for suit in Suit::ALL {
            for rank in Rank::ALL {
                cards.push(Card::Minor { suit, rank });
            }
        }
        Deck { cards }
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    /// Shuffle the deck in place.
    pub fn shuffle<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        self.cards.shuffle(rng);
    }

    /// Draw up to `n` cards from the top, assigning each a random (50/50)
    /// orientation. Returns fewer than `n` only if the deck runs out.
    pub fn draw<R: Rng + ?Sized>(&mut self, rng: &mut R, n: usize) -> Vec<DrawnCard> {
        let n = n.min(self.cards.len());
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let card = self.cards.pop().expect("length checked above");
            let orientation = if rng.gen_bool(0.5) {
                Orientation::Upright
            } else {
                Orientation::Reversed
            };
            out.push(DrawnCard { card, orientation });
        }
        out
    }
}

impl Default for Deck {
    fn default() -> Self {
        Self::full()
    }
}
