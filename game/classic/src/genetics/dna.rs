//! A person's genes as Luna keeps them (`/datum/dna`): strings of hex digits in blocks of three.
//! The unique identifier (13 blocks) is how they look; the structural enzymes (25 blocks) are
//! their powers and defects; the unique enzymes name who they are.

use rs13_world::{FACIAL_STYLES, HAIR_STYLES, HumanLook};

use crate::rng::GameRng;

/// Blocks of the unique identifier and of the structural enzymes (Luna `STRUCDNASIZE`).
pub const UI_BLOCKS: usize = 13;
pub const SE_BLOCKS: usize = 25;
/// The structural block that makes a monkey of whoever has it on.
pub const MONKEY_BLOCK: usize = 14;
/// The unique identifier's block for being a woman.
pub const GENDER_BLOCK: usize = 11;
/// A block's highest value, `FFF`.
pub const BLOCK_MAX: i32 = 4095;

/// Luna's hair list for the identifier: bald first, then the styles in `HAIR_STYLES`' order.
const HAIR_KINDS: i32 = HAIR_STYLES.len() as i32 + 1;
/// Luna's beards, each with its 300-wide stretch of the beard block, shaved first (1 to 350).
const BEARDS: [&str; 13] = [
    "facial_elvis",
    "facial_vandyke",
    "facial_neckbeard",
    "facial_chaplin",
    "facial_watson",
    "facial_abe",
    "facial_chin",
    "facial_hip",
    "facial_gt",
    "facial_hogan",
    "facial_selleck",
    "facial_fullbeard",
    "facial_longbeard",
];

/// Three hex digits.
pub fn hex3(value: i32) -> String {
    format!("{:03X}", value.clamp(0, BLOCK_MAX))
}

/// Block `n` (from 1) of a gene string, as a number.
pub fn block(genes: &str, n: usize) -> i32 {
    genes
        .get((n - 1) * 3..n * 3)
        .and_then(|b| i32::from_str_radix(b, 16).ok())
        .unwrap_or(0)
}

/// Block `n` (from 1), as its three digits.
pub fn block_text(genes: &str, n: usize) -> &str {
    genes.get((n - 1) * 3..n * 3).unwrap_or("000")
}

/// Replace block `n` (from 1) with three digits.
pub fn set_block(genes: &mut String, n: usize, digits: &str) {
    if digits.len() == 3 && n >= 1 && n * 3 <= genes.len() {
        genes.replace_range((n - 1) * 3..n * 3, digits);
    }
}

/// Replace one digit (`sub` from 1 to 3) of block `n`.
pub fn set_digit(genes: &mut String, n: usize, sub: usize, digit: char) {
    let at = (n - 1) * 3 + sub - 1;
    if (1..=3).contains(&sub) && at < genes.len() {
        genes.replace_range(at..=at, &digit.to_string());
    }
}

/// One digit (`sub` from 1 to 3) of block `n`.
pub fn digit(genes: &str, n: usize, sub: usize) -> char {
    genes.chars().nth((n - 1) * 3 + sub - 1).unwrap_or('0')
}

/// Luna `toggledblock`: an off block turned on, or an on one off, at random.
pub fn toggled(value: i32, rng: &mut GameRng) -> i32 {
    if value < 2050 {
        rng.range(2050, BLOCK_MAX)
    } else {
        rng.range(1, 2049)
    }
}

/// Luna's `md5(real_name)`, as 32 hex digits: the same name always gives the same enzymes.
pub fn unique_enzymes(name: &str) -> String {
    // Two FNV-1a passes with different seeds, for 128 bits.
    let mut out = String::with_capacity(32);
    for seed in [0xcbf2_9ce4_8422_2325_u64, 0x8422_2325_cbf2_9ce4] {
        let mut h = seed;
        for byte in name.bytes() {
            h ^= u64::from(byte);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        out.push_str(&format!("{h:016x}"));
    }
    out
}

/// Luna `ready_dna`'s identifier: how someone looks, as 13 blocks. The beard and hair blocks
/// land somewhere in their style's stretch.
pub fn identity_of(look: &HumanLook, block_add: i32, rng: &mut GameRng) -> String {
    let [r, g, b] = look.hair_rgb;
    let [er, eg, eb] = look.eyes_rgb;
    let mut ui = String::with_capacity(UI_BLOCKS * 3);
    // Hair, then facial hair: red, blue, green, as Luna stores them.
    for _ in 0..2 {
        for c in [r, b, g] {
            ui.push_str(&hex3(i32::from(c)));
        }
    }
    ui.push_str(&hex3((i32::from(look.skin) + 220) * 16));
    for c in [er, eg, eb] {
        ui.push_str(&hex3(i32::from(c)));
    }
    let gender = if look.female {
        rng.range(2051 + block_add, 4094)
    } else {
        // Luna goes to 2050 + the shift, which reads as a woman one time in two thousand.
        rng.range(1, 2049 + block_add)
    };
    ui.push_str(&hex3(gender));
    ui.push_str(&hex3(beard_value(look.facial, rng)));
    let range = BLOCK_MAX / HAIR_KINDS;
    let style = i32::from(look.hair) + 1;
    ui.push_str(&hex3(style * range + range - rng.range(1, range - 1)));
    ui
}

fn beard_value(facial: u8, rng: &mut GameRng) -> i32 {
    let name = FACIAL_STYLES.get(usize::from(facial).wrapping_sub(1));
    match name.and_then(|n| BEARDS.iter().position(|b| b == n)) {
        // Elvis is 351 to 650, and so on, the last to 4095.
        Some(i) => {
            let low = 351 + 300 * i as i32;
            let high = if i + 1 == BEARDS.len() {
                BLOCK_MAX
            } else {
                low + 299
            };
            rng.range(low, high)
        }
        None => rng.range(1, 350),
    }
}

/// Luna `updateappearance`: the look an identifier makes. Underwear isn't in the genes.
pub fn look_of(ui: &str, block_add: i32, underwear: u8) -> HumanLook {
    let byte = |n: usize| block(ui, n).clamp(0, 255) as u8;
    let skin = (block(ui, 7) / 16 - 220).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
    let beard = block(ui, 12);
    let facial = if beard <= 350 {
        0
    } else {
        let i = (((beard - 351) / 300) as usize).min(BEARDS.len() - 1);
        FACIAL_STYLES
            .iter()
            .position(|f| *f == BEARDS[i])
            .map_or(0, |p| p as u8 + 1)
    };
    let range = BLOCK_MAX / HAIR_KINDS;
    let style = (block(ui, 13) / range).clamp(1, HAIR_KINDS);
    HumanLook {
        female: is_on(block(ui, GENDER_BLOCK), Threshold::Normal, block_add),
        skin,
        hair: (style - 1) as u8,
        facial,
        hair_rgb: [byte(1), byte(3), byte(2)],
        eyes_rgb: [byte(8), byte(9), byte(10)],
        underwear,
    }
}

/// How high a block must be to be on (Luna `isblockon`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Threshold {
    /// Hulk and telekinesis.
    Hard,
    /// X-ray and cold resistance.
    Mid,
    Normal,
}

pub fn is_on(value: i32, threshold: Threshold, block_add: i32) -> bool {
    let base = match threshold {
        Threshold::Hard => 3500,
        Threshold::Mid => 3050,
        Threshold::Normal => 2050,
    };
    value >= base + block_add
}

/// Fresh structural enzymes, every block off (Luna: each 1 to 1024). A monkey's monkey block is on.
pub fn fresh_enzymes(monkey: bool, rng: &mut GameRng) -> String {
    let mut se = String::with_capacity(SE_BLOCKS * 3);
    for n in 1..=SE_BLOCKS {
        let value = if monkey && n == MONKEY_BLOCK {
            rng.range(2050, BLOCK_MAX)
        } else {
            rng.range(1, 1024)
        };
        se.push_str(&hex3(value));
    }
    se
}

fn weighted(choices: &[(i32, char)], rng: &mut GameRng) -> Option<char> {
    let total: i32 = choices.iter().map(|(w, _)| (*w).max(0)).sum();
    if total <= 0 {
        return None;
    }
    let mut roll = rng.range(1, total);
    for (w, c) in choices {
        let w = (*w).max(0);
        if roll <= w {
            return Some(*c);
        }
        roll -= w;
    }
    None
}

/// Luna `miniscramble`: a digit irradiated. Stronger beams push it up, longer ones keep it lower.
pub fn miniscramble(input: char, strength: i32, duration: i32, rng: &mut GameRng) -> char {
    let (s, d) = (strength, duration);
    let ten = s * 10;
    let five = s * 5 + d;
    let choices: Vec<(i32, char)> = match input.to_ascii_uppercase() {
        'C' | 'D' | 'E' | 'F' => vec![
            (ten, '4'),
            (ten, '5'),
            (ten, '6'),
            (ten, '7'),
            (five, '0'),
            (five, '1'),
            (ten - d, '2'),
            (ten - d, '3'),
        ],
        '4'..='9' | 'A' | 'B' => vec![
            (ten, '4'),
            (ten, '5'),
            (ten, 'A'),
            (ten, 'B'),
            (five, 'C'),
            (five, 'D'),
            (five, '2'),
            (five, '3'),
        ],
        _ => vec![
            (ten, '8'),
            (ten, '9'),
            (ten, 'A'),
            (ten, 'B'),
            (ten - d, 'C'),
            (ten - d, 'D'),
            (five, 'E'),
            (five, 'F'),
        ],
    };
    weighted(&choices, rng).unwrap_or('5')
}

/// Luna `miniscrambletarget`: a digit irradiated towards a chosen value, landing on it or near.
pub fn miniscramble_target(target: u8, strength: i32, duration: i32, rng: &mut GameRng) -> char {
    let (s, d) = (strength, duration);
    let t = i32::from(target.min(15));
    // Up to three either side: the nearest likeliest, the furthest least.
    let mut choices = Vec::new();
    for offset in -3..=3_i32 {
        let v = t + offset;
        if !(0..=15).contains(&v) {
            continue;
        }
        let weight = match offset.abs() {
            0 | 1 => s * 10 + d,
            2 => s * 10,
            _ => s * 10 - d,
        };
        let digit = char::from_digit(v as u32, 16).map_or('8', |c| c.to_ascii_uppercase());
        choices.push((weight, digit));
    }
    weighted(&choices, rng).unwrap_or('8')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_survives_the_round_trip() {
        let mut rng = GameRng::seeded(7);
        for hair in 0..=HAIR_STYLES.len() as u8 {
            for facial in 0..=FACIAL_STYLES.len() as u8 {
                let look = HumanLook {
                    female: hair % 2 == 0,
                    skin: -40,
                    hair,
                    facial,
                    hair_rgb: [10, 200, 30],
                    eyes_rgb: [1, 2, 255],
                    underwear: 3,
                };
                for add in [-300, 0, 100] {
                    let ui = identity_of(&look, add, &mut rng);
                    assert_eq!(ui.len(), UI_BLOCKS * 3);
                    assert_eq!(look_of(&ui, add, 3), look, "{ui}");
                }
            }
        }
    }

    #[test]
    fn blocks_read_and_write() {
        let mut genes = "000123FFF".to_owned();
        assert_eq!(block(&genes, 2), 0x123);
        set_block(&mut genes, 1, "ABC");
        set_digit(&mut genes, 3, 2, '0');
        assert_eq!(genes, "ABC123F0F");
        assert_eq!(digit(&genes, 2, 3), '3');
    }
}
