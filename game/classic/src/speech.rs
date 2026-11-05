//! Saying things out loud. Spec: `docs/reference/speech.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{DisplayName, GridPos, MobStat, Opaque, TileMap, TilePos};

use crate::actions::Outbox;
use crate::health::Vitals;
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

/// What can be done with `*` (Luna's `human/emote`, those that take no one to do them to).
pub const EMOTES: [&str; 41] = [
    "blink", "blink-r", "wave", "choke", "clap", "drool", "eyebrow", "chuckle", "twitch", "twitch-s",
    "faint", "cough", "frown", "nod", "blush", "gasp", "giggle", "grin", "cry", "sigh", "laugh", "mumble",
    "grumble", "groan", "howl", "moan", "raise", "shrug", "smile", "shiver", "pale", "tremble", "sneeze",
    "sniff", "snore", "whimper", "wink", "yawn", "collapse", "scream", "point",
];

/// Luna's whisper: heard whole from this far, in part from here, and only seen from here.
pub const WHISPER_RANGE: i32 = 1;
pub const EAVESDROP_RANGE: i32 = 2;
pub const WHISPER_SEEN_RANGE: i32 = 5;

/// What `*something` comes to: the line everyone sees, or `None` if there's no such emote. `*me` and
/// `*custom` are in the player's own words.
pub fn emote(name: &str, typed: &str) -> Option<LocMsg> {
    let (act, rest) = typed.split_once(' ').unwrap_or((typed, ""));
    let act = act.to_lowercase().replace('_', "-");
    if act == "me" || act == "custom" {
        let text: String = rest
            .chars()
            .filter(|c| !c.is_control())
            .take(MAX_MESSAGE_LEN)
            .collect();
        let text = text.trim();
        return (!text.is_empty()).then(|| LocMsg::new("emote-me").arg("name", name).arg("text", text));
    }
    EMOTES
        .contains(&act.as_str())
        .then(|| LocMsg::new(format!("emote-{act}")).arg("name", name))
}

/// What's caught of a whisper from a little too far: about half the words (Luna `Ellipsis`).
pub fn half_heard(text: &str, rng: &mut crate::rng::GameRng) -> String {
    text.split_whitespace()
        .map(|word| if rng.prob(50.0) { "..." } else { word })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Luna's `say_quote`: the verb comes from the last character.
pub fn speech_verb(text: &str) -> &'static str {
    match text.chars().last() {
        Some('?') => "ask",
        Some('!') => "exclaim",
        _ => "say",
    }
}

/// Same deck and no more than `range` tiles away along either axis.
pub fn within_range(from: TilePos, to: TilePos, range: i32) -> bool {
    from.z == to.z && (from.x - to.x).abs() <= range && (from.y - to.y).abs() <= range
}

/// Whether `to` can be seen from `from`: same deck, within `range`, and the straight
/// line between them crosses no opaque tile, nor squeezes diagonally between two that meet
/// at a corner. The end points themselves never block.
pub fn can_see(map: &TileMap, opaque: &[TilePos], from: TilePos, to: TilePos, range: i32) -> bool {
    if !within_range(from, to, range) {
        return false;
    }
    let blocks = |p: TilePos| map.is_opaque(p) || opaque.contains(&p);
    let mut previous: Option<TilePos> = None;
    for p in line(from, to) {
        if p != from && p != to && blocks(p) {
            return false;
        }
        // A diagonal step can't squeeze between two walls that meet at a corner.
        if let Some(q) = previous
            && q.x != p.x
            && q.y != p.y
            && blocks(TilePos::new(p.x, q.y, p.z))
            && blocks(TilePos::new(q.x, p.y, p.z))
        {
            return false;
        }
        previous = Some(p);
    }
    true
}

/// Tiles on the Bresenham line from `a` to `b`, both included.
pub(crate) fn line(a: TilePos, b: TilePos) -> impl Iterator<Item = TilePos> {
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

/// Radio sets, wherever they are: on a tile, in a hand or worn.
type RadioSets<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static crate::radio::Radio,
        Option<&'static GridPos>,
        Option<&'static crate::hands::HeldBy>,
        Option<&'static crate::equipment::WornBy>,
        Has<crate::radio::Intercom>,
    ),
    Without<Mob>,
>;

/// Deliver each pending line to every mob that can see the speaker. A radio prefix sends
/// it into a radio instead, heard directly only next to the speaker; radios with their
/// microphone on pass on what they hear. Every listening radio on the frequency plays it
/// to the mobs near it (`radio.md`).
pub fn speak(
    map: crate::decks::Decks,
    mut speakers: Query<
        (
            Entity,
            &GridPos,
            &DisplayName,
            &mut PendingSpeech,
            Option<&Vitals>,
            Option<&crate::hands::Hands>,
            Option<&crate::equipment::Equipment>,
            Has<crate::combat::Muted>,
        ),
        With<Mob>,
    >,
    mut listeners: Query<
        (
            Entity,
            &GridPos,
            &mut Outbox,
            Option<&Vitals>,
            Has<rs13_world::Ghost>,
        ),
        Or<(With<Mob>, With<rs13_world::Ghost>)>,
    >,
    opaque: Query<&GridPos, (With<Opaque>, Without<Mob>)>,
    radios: RadioSets,
    mut rng: ResMut<crate::rng::GameRng>,
) {
    use crate::radio::{RADIO_TALK_RANGE, RadioMode, format_frequency, split_mode};

    let opaque: Vec<TilePos> = opaque.iter().map(|p| p.0).collect();
    let mob_at: std::collections::HashMap<Entity, TilePos> =
        listeners.iter().map(|(e, p, ..)| (e, p.0)).collect();
    // Where each radio is: its tile, or the tile of whoever holds or wears it.
    let radio_at = |radio: Entity| -> Option<TilePos> {
        let (_, _, pos, held, worn, _) = radios.get(radio).ok()?;
        pos.map(|p| p.0)
            .or_else(|| held.and_then(|h| mob_at.get(&h.0).copied()))
            .or_else(|| worn.and_then(|w| mob_at.get(&w.0).copied()))
    };

    // (speaker, where, range heard directly, message, radio message and its frequencies)
    let mut said: Vec<(Entity, TilePos, i32, LocMsg, Vec<(u16, LocMsg)>)> = Vec::new();
    // (speaker, where, whole, as caught from a little further, as seen from further still)
    let mut whispers: Vec<(Entity, TilePos, LocMsg, LocMsg, LocMsg)> = Vec::new();
    for (speaker, pos, name, mut pending, vitals, hands, equipment, muted) in &mut speakers {
        if muted || vitals.is_some_and(|v| v.stat != MobStat::Conscious) {
            pending.0.clear();
            continue;
        }
        for raw in pending.0.drain(..) {
            if let Some(typed) = raw.trim_start().strip_prefix('*') {
                // Only the one who typed it hears that there's no such thing.
                match emote(&name.0, typed) {
                    Some(msg) => said.push((speaker, pos.0, HEARING_RANGE, msg, Vec::new())),
                    None => said.push((speaker, pos.0, -1, LocMsg::new("emote-unknown"), Vec::new())),
                }
                continue;
            }
            let (mode, rest) = split_mode(&raw);
            let Some(text) = sanitize_speech(rest) else {
                continue;
            };
            if mode == Some(RadioMode::Whisper) {
                let line = |text: &str| {
                    LocMsg::new("whisper")
                        .arg("name", name.0.clone())
                        .arg("text", text)
                };
                let part = half_heard(&text, &mut rng);
                whispers.push((
                    speaker,
                    pos.0,
                    line(&text),
                    line(&part),
                    LocMsg::new("whisper-seen").arg("name", name.0.clone()),
                ));
                continue;
            }
            let verb = speech_verb(&text);
            let msg = LocMsg::new("say")
                .arg("name", name.0.clone())
                .arg("verb", verb)
                .arg("text", text.clone());
            let range = if mode.is_some() {
                RADIO_TALK_RANGE
            } else {
                HEARING_RANGE
            };
            // The radios it goes into: the one named, and any with the microphone on that
            // hear it.
            let mut into: Vec<Entity> = Vec::new();
            match mode {
                Some(RadioMode::RightHand) => into.extend(hands.and_then(|h| h.right)),
                Some(RadioMode::LeftHand) => into.extend(hands.and_then(|h| h.left)),
                Some(RadioMode::Headset) => {
                    into.extend(equipment.and_then(|e| e.get(rs13_world::Slot::Ears)))
                }
                Some(RadioMode::Intercom) => into.extend(
                    radios
                        .iter()
                        .filter(|(_, _, at, _, _, intercom)| {
                            *intercom && at.is_some_and(|p| can_see(map.at(pos.0.z), &opaque, pos.0, p.0, 1))
                        })
                        .map(|(e, ..)| e),
                ),
                Some(RadioMode::Whisper | RadioMode::Channel(_)) | None => {}
            }
            // A department channel, if the headset they're wearing has it.
            let channel = match mode {
                Some(RadioMode::Channel(wanted)) => equipment
                    .and_then(|e| e.get(rs13_world::Slot::Ears))
                    .and_then(|ear| radios.get(ear).ok())
                    .and_then(|(_, radio, ..)| radio.channel(wanted)),
                _ => None,
            };
            into.extend(
                radios
                    .iter()
                    .filter(|(_, radio, at, ..)| {
                        radio.broadcasting
                            && at.is_some_and(|p| can_see(map.at(pos.0.z), &opaque, pos.0, p.0, range))
                    })
                    .map(|(e, ..)| e),
            );
            let mut frequencies: Vec<u16> = into
                .iter()
                .filter_map(|e| radios.get(*e).ok())
                .map(|(_, radio, ..)| radio.frequency)
                .collect();
            frequencies.extend(channel);
            frequencies.sort_unstable();
            frequencies.dedup();
            let on_air = frequencies
                .into_iter()
                .map(|f| {
                    let msg = LocMsg::new("radio-say").arg("name", name.0.clone());
                    // A department channel goes by its name.
                    let msg = match crate::radio::CHANNELS.iter().find(|c| c.0 == f) {
                        Some((_, id, ..)) => msg.msg("frequency", format!("channel-{id}")),
                        None => msg.arg("frequency", format_frequency(f)),
                    };
                    let msg = msg.arg("verb", verb).arg("text", text.clone());
                    (f, msg)
                })
                .collect();
            said.push((speaker, pos.0, range, msg, on_air));
        }
    }
    for (speaker, from, whole, part, seen) in whispers {
        for (listener, to, mut outbox, vitals, _) in &mut listeners {
            if vitals.is_some_and(|v| v.stat != MobStat::Conscious) {
                continue;
            }
            let sees = |range: i32| can_see(map.at(from.z), &opaque, from, to.0, range);
            if listener == speaker || sees(WHISPER_RANGE) {
                outbox.0.push(whole.clone());
            } else if sees(EAVESDROP_RANGE) {
                outbox.0.push(part.clone());
            } else if sees(WHISPER_SEEN_RANGE) {
                outbox.0.push(seen.clone());
            }
        }
    }
    for (speaker, from, range, msg, on_air) in said {
        // Who hears each frequency: anyone near a listening radio on it, once.
        let mut tuned: Vec<(Entity, LocMsg)> = Vec::new();
        for (frequency, radio_msg) in on_air {
            for (radio_entity, radio, ..) in &radios {
                if !radio.hears(frequency) {
                    continue;
                }
                let Some(at) = radio_at(radio_entity) else {
                    continue;
                };
                for (listener, to) in &mob_at {
                    if can_see(map.at(at.z), &opaque, at, *to, radio.range)
                        && !tuned.iter().any(|(e, m)| e == listener && *m == radio_msg)
                    {
                        tuned.push((*listener, radio_msg.clone()));
                    }
                }
            }
        }
        tuned.sort_by_key(|(e, _)| *e);
        for (listener, to, mut outbox, vitals, ghost) in &mut listeners {
            let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            if !awake {
                continue;
            }
            // Ghosts see through walls, and hear through them too.
            let heard = if ghost {
                within_range(from, to.0, range)
            } else {
                can_see(map.at(from.z), &opaque, from, to.0, range)
            };
            if listener == speaker || heard {
                outbox.0.push(msg.clone());
            }
            for (_, radio_msg) in tuned.iter().filter(|(e, _)| *e == listener) {
                outbox.0.push(radio_msg.clone());
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

#[cfg(test)]
mod sight_tests {
    use super::*;

    #[test]
    fn sight_does_not_squeeze_between_walls_that_meet_at_a_corner() {
        // Two walls touching at a corner, diagonally between the viewer and the target.
        let map = TileMap::from_ron(
            r#"(
                legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
                rows: ["...", ".W.", "W..", "..."],
            )"#,
            0,
        )
        .unwrap();
        // Rows run top down: the walls are at (1, 2) and (0, 1).
        let (from, to) = (TilePos::new(0, 2, 0), TilePos::new(1, 1, 0));
        assert!(!can_see(&map, &[], from, to, 7));
        assert!(can_see(
            &map,
            &[],
            TilePos::new(0, 3, 0),
            TilePos::new(2, 3, 0),
            7
        ));
    }
}
