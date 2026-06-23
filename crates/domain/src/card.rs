//! The card model for the Tarot de Marseille: 22 Major Arcana + 56 Minor Arcana.
//!
//! Cards carry a stable [`Card::id`] used to index both the bundled art file and
//! the meanings table, so adding art or text never requires touching this code.

use serde::{Deserialize, Serialize};

/// Whether a drawn card landed upright or reversed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Orientation {
    Upright,
    Reversed,
}

impl Orientation {
    /// Lower-case label, e.g. for prompts or logs.
    pub fn label(self) -> &'static str {
        match self {
            Orientation::Upright => "upright",
            Orientation::Reversed => "reversed",
        }
    }
}

/// The four Minor Arcana suits, in Tarot de Marseille (French) naming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    Coupes,  // Cups
    Deniers, // Coins / Pentacles
    Epees,   // Swords
    Batons,  // Batons / Wands
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Coupes, Suit::Deniers, Suit::Epees, Suit::Batons];

    /// ASCII key used in [`Card::id`] and asset filenames.
    pub fn key(self) -> &'static str {
        match self {
            Suit::Coupes => "coupes",
            Suit::Deniers => "deniers",
            Suit::Epees => "epees",
            Suit::Batons => "batons",
        }
    }

    /// French display name.
    pub fn display(self) -> &'static str {
        match self {
            Suit::Coupes => "Coupes",
            Suit::Deniers => "Deniers",
            Suit::Epees => "Épées",
            Suit::Batons => "Bâtons",
        }
    }
}

/// The fourteen ranks of a Minor Arcana suit: Ace through Ten, then the four
/// court cards (Valet, Cavalier, Reine, Roi).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rank {
    Ace,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Valet,
    Cavalier,
    Reine,
    Roi,
}

impl Rank {
    pub const ALL: [Rank; 14] = [
        Rank::Ace,
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Valet,
        Rank::Cavalier,
        Rank::Reine,
        Rank::Roi,
    ];

    /// ASCII key used in [`Card::id`] and asset filenames.
    pub fn key(self) -> &'static str {
        match self {
            Rank::Ace => "ace",
            Rank::Two => "two",
            Rank::Three => "three",
            Rank::Four => "four",
            Rank::Five => "five",
            Rank::Six => "six",
            Rank::Seven => "seven",
            Rank::Eight => "eight",
            Rank::Nine => "nine",
            Rank::Ten => "ten",
            Rank::Valet => "valet",
            Rank::Cavalier => "cavalier",
            Rank::Reine => "reine",
            Rank::Roi => "roi",
        }
    }

    /// French display name.
    pub fn display(self) -> &'static str {
        match self {
            Rank::Ace => "As",
            Rank::Two => "Deux",
            Rank::Three => "Trois",
            Rank::Four => "Quatre",
            Rank::Five => "Cinq",
            Rank::Six => "Six",
            Rank::Seven => "Sept",
            Rank::Eight => "Huit",
            Rank::Nine => "Neuf",
            Rank::Ten => "Dix",
            Rank::Valet => "Valet",
            Rank::Cavalier => "Cavalier",
            Rank::Reine => "Reine",
            Rank::Roi => "Roi",
        }
    }
}

/// The 22 Major Arcana: (name, Tarot de Marseille numeral). Indexed by number 0..=21.
/// Numerals use the additive Marseille form (IIII, VIIII, XIIII, XVIIII).
/// Le Mat is traditionally unnumbered, so its numeral is empty.
const MAJORS: [(&str, &str); 22] = [
    ("Le Mat", ""),
    ("Le Bateleur", "I"),
    ("La Papesse", "II"),
    ("L'Impératrice", "III"),
    ("L'Empereur", "IIII"),
    ("Le Pape", "V"),
    ("L'Amoureux", "VI"),
    ("Le Chariot", "VII"),
    ("La Justice", "VIII"),
    ("L'Ermite", "VIIII"),
    ("La Roue de Fortune", "X"),
    ("La Force", "XI"),
    ("Le Pendu", "XII"),
    ("La Mort", "XIII"),
    ("Tempérance", "XIIII"),
    ("Le Diable", "XV"),
    ("La Maison Dieu", "XVI"),
    ("L'Étoile", "XVII"),
    ("La Lune", "XVIII"),
    ("Le Soleil", "XVIIII"),
    ("Le Jugement", "XX"),
    ("Le Monde", "XXI"),
];

/// A single tarot card. Either a numbered Major Arcanum (0..=21) or a Minor
/// Arcanum identified by suit + rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Card {
    Major { number: u8 },
    Minor { suit: Suit, rank: Rank },
}

impl Card {
    /// Stable identifier used to look up meanings and art.
    ///
    /// Majors: `major_00` .. `major_21`. Minors: `<suit>_<rank>`, e.g. `coupes_ace`.
    pub fn id(self) -> String {
        match self {
            Card::Major { number } => format!("major_{number:02}"),
            Card::Minor { suit, rank } => format!("{}_{}", suit.key(), rank.key()),
        }
    }

    /// Human-readable name, e.g. "XIII — La Mort" or "As de Coupes".
    pub fn display_name(self) -> String {
        match self {
            Card::Major { number } => {
                let (name, numeral) = MAJORS[number as usize];
                if numeral.is_empty() {
                    name.to_string()
                } else {
                    format!("{numeral} — {name}")
                }
            }
            Card::Minor { suit, rank } => {
                format!("{} de {}", rank.display(), suit.display())
            }
        }
    }

    /// Relative path (under the Bevy `assets/` dir) to this card's face art.
    pub fn asset_path(self) -> String {
        format!("cards/{}.png", self.id())
    }
}
