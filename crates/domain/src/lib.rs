//! Engine-agnostic Tarot de Marseille domain logic.
//!
//! This crate has no rendering or engine dependencies — it is the testable core:
//! the deck, shuffling and drawing, the spread abstraction (Celtic Cross first),
//! the authored meanings table, and reading composition. The Bevy app layers
//! presentation and animation on top of this.

pub mod card;
pub mod deck;
pub mod meanings;
pub mod reading;
pub mod spread;

pub mod spreads {
    pub mod celtic_cross;
}

pub use card::{Card, Orientation, Rank, Suit};
pub use deck::{Deck, DrawnCard};
pub use meanings::{CardMeaning, Meanings};
pub use reading::{build_reading, Reading, ReadingEntry};
pub use spread::{LayoutSlot, PositionDef, Spread};
pub use spreads::celtic_cross::CelticCross;
