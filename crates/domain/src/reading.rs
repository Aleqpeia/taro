//! Composing a [`Reading`]: pairing each spread position with its drawn card and
//! the authored meaning for that card's orientation.

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
