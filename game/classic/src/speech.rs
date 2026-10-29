//! Saying things out loud. Spec: `docs/reference/speech.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{DisplayName, GridPos, MapLayer, Opaque, TileMap, TilePos};

use crate::actions::Outbox;
use crate::movement::Mob;

/// Longest message, in characters (Luna `MAX_MESSAGE_LEN`).
pub const MAX_MESSAGE_LEN: usize = 1024;
/// How far normal speech carries: half of Luna's 15×15 view.
pub const HEARING_RANGE: i32 = 7;

/// Lines this mob wants to say, waiting for the next tick.
#[derive(Component, Debug, Default)]
pub struct PendingSpeech(pub Vec<String>);

/// Clean up what a player typed: drop control characters, cut to the length limit,
/// trim, and capitalise the first letter. `None` if nothing is left.
pub fn sanitize_speech(input: &str) -> Option<String> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_MESSAGE_LEN)
        .collect();
    let trimmed = cleaned.trim();
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    Some(first.to_uppercase().chain(chars).collect())
}

/// Luna's `say_quote`: the verb comes from the last character.
pub fn speech_verb(text: &str) -> &'static str {
    match text.chars().last() {
        Some('?') => "ask",
        Some('!') => "exclaim",
        _ => "say",
    }
}

/// Whether `to` can be seen from `from`: same deck, within `range`, and the straight
/// line between them crosses no opaque tile. The end points themselves never block.
pub fn can_see(map: &TileMap, opaque: &[TilePos], from: TilePos, to: TilePos, range: i32) -> bool {
    if from.z != to.z || (from.x - to.x).abs() > range || (from.y - to.y).abs() > range {
        return false;
    }
    line(from, to)
        .filter(|p| *p != from && *p != to)
        .all(|p| !map.is_opaque(p) && !opaque.contains(&p))
}

/// Tiles on the Bresenham line from `a` to `b`, both included.
fn line(a: TilePos, b: TilePos) -> impl Iterator<Item = TilePos> {
    let (dx, dy) = ((b.x - a.x).abs(), -(b.y - a.y).abs());
    let (sx, sy) = ((b.x - a.x).signum(), (b.y - a.y).signum());
    let mut err = dx + dy;
    let mut p = Some(a);
    std::iter::from_fn(move || {
        let current = p?;
        p = if current.x == b.x && current.y == b.y {
            None
        } else {
            let mut next = current;
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                next.x += sx;
            }
            if e2 <= dx {
                err += dx;
                next.y += sy;
            }
            Some(next)
        };
        Some(current)
    })
}

/// Deliver each pending line to every mob that can see the speaker.
pub fn speak(
    map: Single<&MapLayer>,
    mut speakers: Query<(Entity, &GridPos, &DisplayName, &mut PendingSpeech), With<Mob>>,
    mut listeners: Query<(Entity, &GridPos, &mut Outbox), With<Mob>>,
    opaque: Query<&GridPos, (With<Opaque>, Without<Mob>)>,
) {
    let opaque: Vec<TilePos> = opaque.iter().map(|p| p.0).collect();
    let mut said: Vec<(Entity, TilePos, LocMsg)> = Vec::new();
    for (speaker, pos, name, mut pending) in &mut speakers {
        for raw in pending.0.drain(..) {
            let Some(text) = sanitize_speech(&raw) else {
                continue;
            };
            let msg = LocMsg::new("say")
                .arg("name", name.0.clone())
                .arg("verb", speech_verb(&text))
                .arg("text", text);
            said.push((speaker, pos.0, msg));
        }
    }
    for (speaker, from, msg) in said {
        for (listener, to, mut outbox) in &mut listeners {
            if listener == speaker || can_see(&map.0, &opaque, from, to.0, HEARING_RANGE) {
                outbox.0.push(msg.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_is_cleaned_and_capitalised() {
        assert_eq!(sanitize_speech("  hello there ").as_deref(), Some("Hello there"));
        assert_eq!(sanitize_speech("привет").as_deref(), Some("Привет"));
        assert_eq!(sanitize_speech("a\u{7}b\nc").as_deref(), Some("Abc"));
        assert_eq!(sanitize_speech("   "), None);
        assert_eq!(
            sanitize_speech(&"x".repeat(2000)).unwrap().chars().count(),
            MAX_MESSAGE_LEN
        );
    }

    #[test]
    fn verb_follows_the_last_character() {
        assert_eq!(speech_verb("Hello."), "say");
        assert_eq!(speech_verb("Who's there?"), "ask");
        assert_eq!(speech_verb("Help!"), "exclaim");
    }

    #[test]
    fn line_includes_both_ends() {
        let pts: Vec<_> = line(TilePos::new(0, 0, 0), TilePos::new(3, 1, 0)).collect();
        assert_eq!(pts.first(), Some(&TilePos::new(0, 0, 0)));
        assert_eq!(pts.last(), Some(&TilePos::new(3, 1, 0)));
        assert_eq!(pts.len(), 4);
    }
}
