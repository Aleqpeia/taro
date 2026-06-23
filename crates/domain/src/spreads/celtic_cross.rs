//! The Celtic Cross — a ten-card spread: a six-card cross plus a four-card staff.
//!
//! Layout grid (rows increase downward):
//!
//! ```text
//!            [5]
//!                            [10]
//!   [4]   [1x2]   [6]        [ 9]
//!                            [ 8]
//!            [3]             [ 7]
//! ```
//!
//! Position 2 (the Challenge) is laid across position 1 (`rotated: true`),
//! occupying the same slot.

use crate::spread::{LayoutSlot, PositionDef, Spread};

/// The Celtic Cross spread.
pub struct CelticCross;

const POSITIONS: [PositionDef; 10] = [
    PositionDef {
        index: 1,
        name: "The Present",
        description: "The heart of the matter — the querent's current situation.",
        slot: LayoutSlot { col: 1, row: 1, rotated: false },
    },
    PositionDef {
        index: 2,
        name: "The Challenge",
        description: "The obstacle crossing the situation, for good or ill.",
        slot: LayoutSlot { col: 1, row: 1, rotated: true },
    },
    PositionDef {
        index: 3,
        name: "The Foundation",
        description: "The root of the matter — the recent past or underlying cause.",
        slot: LayoutSlot { col: 1, row: 2, rotated: false },
    },
    PositionDef {
        index: 4,
        name: "The Past",
        description: "Influences receding — what is passing away.",
        slot: LayoutSlot { col: 0, row: 1, rotated: false },
    },
    PositionDef {
        index: 5,
        name: "The Crown",
        description: "The best that can be achieved — a possible outcome or aim.",
        slot: LayoutSlot { col: 1, row: 0, rotated: false },
    },
    PositionDef {
        index: 6,
        name: "The Near Future",
        description: "What is approaching in the immediate days ahead.",
        slot: LayoutSlot { col: 2, row: 1, rotated: false },
    },
    PositionDef {
        index: 7,
        name: "The Self",
        description: "The querent's own attitude and role in the situation.",
        slot: LayoutSlot { col: 4, row: 3, rotated: false },
    },
    PositionDef {
        index: 8,
        name: "Environment",
        description: "External influences — other people, surroundings, circumstances.",
        slot: LayoutSlot { col: 4, row: 2, rotated: false },
    },
    PositionDef {
        index: 9,
        name: "Hopes & Fears",
        description: "The querent's inner hopes and anxieties about the matter.",
        slot: LayoutSlot { col: 4, row: 1, rotated: false },
    },
    PositionDef {
        index: 10,
        name: "The Outcome",
        description: "The culmination — where the current path leads.",
        slot: LayoutSlot { col: 4, row: 0, rotated: false },
    },
];

impl Spread for CelticCross {
    fn name(&self) -> &str {
        "Celtic Cross"
    }

    fn positions(&self) -> &[PositionDef] {
        &POSITIONS
    }
}
