//! Steno strokes as bits, in Plover's notation (`KAT`, `-T`, `STKPW`, `TKPWHR*EUFPLT`).

use std::fmt;

/// The keys in steno order. Index is the bit.
const KEYS: &[u8; 23] = b"#STKPWHRAO*EUFRPBLGTSDZ";
/// Index of the first key on the right hand: `F`.
const RIGHT: u32 = 13;
/// Index of the first key of the vowel block: `A`.
const VOWEL: u32 = 8;

/// Which keys of a stroke are down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Stroke(pub u32);

const fn mask(from: u32, to: u32) -> u32 {
    ((1u32 << (to - from + 1)) - 1) << from
}

impl Stroke {
    pub const LEFT: u32 = mask(1, 7);
    /// `A`, `O`, `E`, `U`, without the star.
    pub const VOWELS: u32 = mask(8, 9) | mask(11, 12);
    pub const STAR: u32 = 1 << 10;
    pub const RIGHT: u32 = mask(RIGHT, 22);
    /// The join key: the stroke continues the word before it. Written `+` before the keys.
    pub const JOIN: u32 = 1 << 23;
    /// The spelling-mode marker: this stroke starts a run read letter-by-letter from
    /// `rules/spelling.toml` instead of sounded out. Written `~` before the keys, only ever on
    /// the first stroke of such a run — every stroke after it carries the ordinary join key
    /// instead, the same way any other multi-stroke word continues. See `docs/spelling-mode.md`.
    pub const SPELL: u32 = 1 << 24;

    /// Parse notation such as `KAT`, `-T` or `STPH*`. Numbers and a bare `#` are not supported.
    pub fn parse(text: &str) -> Option<Self> {
        let (mut bits, text) = if let Some(rest) = text.strip_prefix('+') {
            (Self::JOIN, rest)
        } else if let Some(rest) = text.strip_prefix('~') {
            (Self::SPELL, rest)
        } else {
            (0, text)
        };
        let mut cursor = 0usize;
        for c in text.bytes() {
            if c == b'-' {
                cursor = cursor.max(RIGHT as usize);
                continue;
            }
            let at = KEYS[cursor..].iter().position(|&k| k == c)? + cursor;
            bits |= 1 << at;
            cursor = at + 1;
        }
        (bits & !(Self::JOIN | Self::SPELL) != 0).then_some(Self(bits))
    }

    /// The lowest key down, as an index in steno order.
    pub const fn lowest(self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            Some(self.0.trailing_zeros())
        }
    }

    /// The highest key down.
    pub const fn highest(self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            Some(31 - self.0.leading_zeros())
        }
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Stroke {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let has_vowel = self.0 & (mask(VOWEL, 12)) != 0;
        let mut hyphen = !has_vowel;
        if self.0 & Self::JOIN != 0 {
            write!(f, "+")?;
        } else if self.0 & Self::SPELL != 0 {
            write!(f, "~")?;
        }
        for (index, &key) in KEYS.iter().enumerate() {
            if self.0 & (1 << index) == 0 {
                continue;
            }
            if hyphen && index as u32 >= RIGHT {
                write!(f, "-")?;
                hyphen = false;
            }
            write!(f, "{}", key as char)?;
        }
        Ok(())
    }
}

/// Parse `KAT/TKOG` into its strokes.
pub fn parse_outline(text: &str) -> Option<Vec<Stroke>> {
    text.split('/').map(Stroke::parse).collect()
}

pub fn render_outline(strokes: &[Stroke]) -> String {
    strokes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notation_round_trips() {
        for text in [
            "KAT",
            "-T",
            "STPH",
            "TKPWHR*EUFPLT",
            "AOEU",
            "*S",
            "SKWR-PBLG",
            "U",
            "+-G",
            "+AOE",
            "~S",
            "~TKPW",
        ] {
            assert_eq!(Stroke::parse(text).unwrap().to_string(), text, "{text}");
        }
    }

    #[test]
    fn a_hyphen_puts_keys_on_the_right() {
        assert_eq!(Stroke::parse("T"), Stroke::parse("T"));
        assert_ne!(Stroke::parse("T"), Stroke::parse("-T"));
        assert_eq!(Stroke::parse("-T").unwrap().to_string(), "-T");
    }

    #[test]
    fn keys_out_of_order_do_not_parse() {
        assert_eq!(Stroke::parse("TS-K"), None);
    }

    #[test]
    fn the_spelling_marker_is_not_the_join_key() {
        let spell = Stroke::parse("~S").unwrap();
        let join = Stroke::parse("+S").unwrap();
        assert_ne!(spell, join);
        assert_eq!(spell.0 & Stroke::SPELL, Stroke::SPELL);
        assert_eq!(spell.0 & Stroke::JOIN, 0);
        // A bare marker with no keys down is as meaningless as a bare join key alone
        assert_eq!(Stroke::parse("~"), None);
        assert_eq!(Stroke::parse("+"), None);
    }
}
