//! Spread abstraction. A spread is an ordered list of positions, each with a
//! name, an interpretive role, and a layout slot. New spreads are added by
//! implementing [`Spread`] — the reading engine and (later) the renderer are
//! spread-agnostic.

/// A grid coordinate for a position, mapped to a world transform by the
/// renderer. `rotated` marks a card laid sideways (the Celtic Cross "crossing"
/// card). Rows increase downward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutSlot {
    pub col: i32,
    pub row: i32,
    pub rotated: bool,
}

/// One position within a spread.
#[derive(Debug, Clone, Copy)]
pub struct PositionDef {
    /// 1-based position number as traditionally dealt.
    pub index: usize,
    /// Short title, e.g. "The Challenge".
    pub name: &'static str,
    /// What this position represents in the reading.
    pub description: &'static str,
    /// Where the card sits in the layout.
    pub slot: LayoutSlot,
}

/// A tarot spread: an ordered set of positions.
pub trait Spread {
    fn name(&self) -> &str;
    fn positions(&self) -> &[PositionDef];

    /// Number of cards this spread requires.
    fn card_count(&self) -> usize {
        self.positions().len()
    }
}
