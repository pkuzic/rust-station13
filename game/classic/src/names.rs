//! Character names. Spec: `docs/reference/speech.md` (names section).

/// Longest accepted name, in characters.
pub const MAX_NAME_LEN: usize = 26;

/// Names that read as part of the map and are refused (case-insensitive).
const RESERVED: [&str; 7] = [
    "space",
    "floor",
    "wall",
    "r-wall",
    "monkey",
    "unknown",
    "inactive ai",
];

/// Clean up a requested name following Luna's `reject_bad_name`, or `None` if it is
/// not acceptable. Letters only, with `' - .` and single spaces between words; each
/// word starts upper case; at least two letters.
pub fn sanitize_name(input: &str) -> Option<String> {
    if input.chars().count() > MAX_NAME_LEN {
        return None;
    }
    #[derive(PartialEq, PartialOrd)]
    enum Last {
        Start,
        Space,
        Punct,
        Letter,
    }
    let mut out = String::new();
    let mut last = Last::Start;
    let mut letters = 0;
    for c in input.chars() {
        match c {
            'A'..='Z' => {
                out.push(c);
                letters += 1;
                last = Last::Letter;
            }
            'a'..='z' => {
                out.push(if last < Last::Punct {
                    c.to_ascii_uppercase()
                } else {
                    c
                });
                letters += 1;
                last = Last::Letter;
            }
            // Digits are dropped for human names.
            '0'..='9' => {}
            '\'' | '-' | '.' => {
                if last != Last::Start {
                    out.push(c);
                    last = Last::Punct;
                }
            }
            ' ' => {
                if last > Last::Space {
                    out.push(' ');
                    last = Last::Space;
                }
            }
            _ => return None,
        }
    }
    if letters < 2 {
        return None;
    }
    if last == Last::Space {
        out.pop();
    }
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(&out)) {
        return None;
    }
    Some(out)
}

const FIRST: [&str; 24] = [
    "Alan", "Beatrice", "Carl", "Dana", "Edgar", "Fiona", "Gordon", "Helen", "Ivan", "Judith", "Kurt",
    "Lena", "Martin", "Nora", "Oscar", "Paula", "Quentin", "Rita", "Simon", "Tamara", "Victor", "Wanda",
    "Yuri", "Zoe",
];

const LAST: [&str; 24] = [
    "Abbott",
    "Barnes",
    "Carver",
    "Dorsey",
    "Ellis",
    "Fowler",
    "Grant",
    "Holt",
    "Irwin",
    "Jensen",
    "Keller",
    "Lowe",
    "Mercer",
    "Nolan",
    "Orton",
    "Pryce",
    "Quill",
    "Rowe",
    "Sutton",
    "Talbot",
    "Underwood",
    "Vance",
    "Webb",
    "Young",
];

/// A default name for someone who has not chosen one, derived from `seed` so it is
/// stable for the same mob.
pub fn default_name(seed: u64) -> String {
    let first = FIRST[(seed.wrapping_mul(7) % FIRST.len() as u64) as usize];
    let last = LAST[(seed.wrapping_mul(13).wrapping_add(5) % LAST.len() as u64) as usize];
    format!("{first} {last}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_up_names_like_luna() {
        assert_eq!(sanitize_name("john smith").as_deref(), Some("John Smith"));
        assert_eq!(
            sanitize_name("  Mary   Ann O'Neil ").as_deref(),
            Some("Mary Ann O'Neil")
        );
        assert_eq!(sanitize_name("R2D2 Unit").as_deref(), Some("RD Unit"));
        assert_eq!(sanitize_name("Jean-luc").as_deref(), Some("Jean-luc"));
    }

    #[test]
    fn rejects_bad_names() {
        assert_eq!(sanitize_name("Иван Петров"), None, "names are English-only");
        assert_eq!(sanitize_name("A"), None);
        assert_eq!(sanitize_name("' ' ' '"), None);
        assert_eq!(sanitize_name("Unknown"), None);
        assert_eq!(sanitize_name("Space"), None);
        assert_eq!(sanitize_name("John <b>Smith</b>"), None);
        assert_eq!(sanitize_name(&"a".repeat(27)), None);
    }

    #[test]
    fn default_names_are_valid() {
        for seed in 0..100 {
            let name = default_name(seed);
            assert_eq!(sanitize_name(&name), Some(name.clone()));
        }
    }
}
