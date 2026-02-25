use crate::error::{IxError, Result};
use crate::index::Index;
use crate::item::Item;

#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    Slot(usize),
    Range(usize, usize),
    NegativeSlot(usize),
    NegativeRange(usize, usize),
}

#[derive(Debug, Clone)]
pub struct Selection(pub Vec<Selector>);

impl Selection {
    /// Expand all selectors into a flat, ordered list of slot numbers.
    pub fn slots(&self) -> Vec<usize> {
        let mut included = Vec::new();
        let mut excluded = Vec::new();

        for sel in &self.0 {
            match sel {
                Selector::Slot(n) => included.extend(*n..=*n),
                Selector::Range(s, e) => included.extend(*s..=*e),
                Selector::NegativeSlot(n) => excluded.extend(*n..=*n),
                Selector::NegativeRange(s, e) => excluded.extend(*s..=*e),
            }
        }

        included
            .into_iter()
            .filter(|n| !excluded.contains(n))
            .collect()
    }

    /// Parses a slice of strings into a `Selection`.
    ///
    /// Supports:
    /// - Individual slots: `"1"`, `"3"`
    /// - Ranges: `"1-4"`
    /// - Negative slots: `"^5"`
    /// - Negative ranges: `"^5-8"`
    /// - Comma-separated: `"1,3,^5"`, `"1,3-5"`
    /// - Mixed: `&["1,3", "^5-7"]`
    pub fn parse(args: &[&str]) -> Result<Self> {
        let mut selectors = Vec::new();

        for arg in args {
            // Split by comma first, then handle each token
            for token in arg.split(',') {
                let token = token.trim();
                if token.is_empty() {
                    continue;
                }
                let sel = parse_token(token)?;
                selectors.push(sel);
            }
        }

        if selectors.is_empty() {
            return Err(IxError::ParseError("No selectors provided".into()));
        }

        Ok(Self(selectors))
    }

    pub fn resolve<'a>(&self, index: &'a Index) -> Result<Vec<&'a Item>> {
        let total = index.items.len();
        let map = index.slot_map();
        self.slots()
            .into_iter()
            .map(|n| {
                map.get(&n)
                    .copied()
                    .ok_or(IxError::SlotOutOfRange(n, total))
            })
            .collect()
    }
}

fn parse_token(token: &str) -> Result<Selector> {
    let (is_negative, token) = if let Some(stripped) = token.strip_prefix('^') {
        (true, stripped)
    } else {
        (false, token)
    };

    if let Some((left, right)) = token.split_once('-') {
        // Could be a range like "1-4"
        let start = left
            .trim()
            .parse::<usize>()
            .map_err(|_| IxError::ParseError(format!("Invalid range start in '{token}'")))?;
        let end = right
            .trim()
            .parse::<usize>()
            .map_err(|_| IxError::ParseError(format!("Invalid range end in '{token}'")))?;
        if start > end {
            return Err(IxError::ParseError(format!(
                "Range start ({start}) must be <= end ({end})"
            )));
        }
        if is_negative {
            Ok(Selector::NegativeRange(start, end))
        } else {
            Ok(Selector::Range(start, end))
        }
    } else {
        let n = token.parse::<usize>().map_err(|_| {
            IxError::ParseError(format!("'{token}' is not a valid slot number or range"))
        })?;
        if is_negative {
            Ok(Selector::NegativeSlot(n))
        } else {
            Ok(Selector::Slot(n))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single() {
        let sel = Selection::parse(&["1"]).unwrap();
        assert_eq!(sel.0, vec![Selector::Slot(1)]);
    }

    #[test]
    fn test_parse_range() {
        let sel = Selection::parse(&["1-3"]).unwrap();
        assert_eq!(sel.0, vec![Selector::Range(1, 3)]);
    }

    #[test]
    fn test_parse_comma() {
        let sel = Selection::parse(&["1,3,5"]).unwrap();
        assert_eq!(
            sel.0,
            vec![Selector::Slot(1), Selector::Slot(3), Selector::Slot(5)]
        );
    }

    #[test]
    fn test_parse_mixed() {
        let sel = Selection::parse(&["1,3", "5-7"]).unwrap();
        assert_eq!(
            sel.0,
            vec![Selector::Slot(1), Selector::Slot(3), Selector::Range(5, 7),]
        );
    }

    #[test]
    fn test_parse_multiple_args() {
        let sel = Selection::parse(&["1", "3"]).unwrap();
        assert_eq!(sel.0, vec![Selector::Slot(1), Selector::Slot(3)]);
    }

    #[test]
    fn test_parse_negative() {
        let sel = Selection::parse(&["^5"]).unwrap();
        assert_eq!(sel.0, vec![Selector::NegativeSlot(5)]);

        let sel2 = Selection::parse(&["^2-4"]).unwrap();
        assert_eq!(sel2.0, vec![Selector::NegativeRange(2, 4)]);
    }

    #[test]
    fn test_selection_parses_range() {
        let sel = Selection::parse(&["1-3"]).unwrap();
        assert_eq!(sel.slots(), vec![1, 2, 3]);
    }

    #[test]
    fn test_selection_parses_comma_separated() {
        let sel = Selection::parse(&["1,3,5"]).unwrap();
        assert_eq!(sel.slots(), vec![1, 3, 5]);
    }

    #[test]
    fn test_selection_parses_mixed() {
        let sel = Selection::parse(&["1,3", "5-7"]).unwrap();
        assert_eq!(sel.slots(), vec![1, 3, 5, 6, 7]);
    }

    #[test]
    fn test_selection_excludes_negatives() {
        let sel = Selection::parse(&["1-5", "^3"]).unwrap();
        assert_eq!(sel.slots(), vec![1, 2, 4, 5]);

        let sel2 = Selection::parse(&["1-10", "^3-5", "^8"]).unwrap();
        assert_eq!(sel2.slots(), vec![1, 2, 6, 7, 9, 10]);
    }
}
