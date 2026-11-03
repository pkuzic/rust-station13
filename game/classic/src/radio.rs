//! Radios and intercoms: talking into one sends speech to every listening radio on its
//! frequency, heard by the people near each. Spec: `docs/reference/radio.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_world::{Appearance, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;

/// The common channel, in BYOND's tenths of a MHz (Luna 1459, shown as 145.9).
pub const COMMON_FREQUENCY: u16 = 1459;
/// How far from a handheld radio its speaker is heard (Luna `hearers(1)`).
pub const RADIO_RANGE: i32 = 1;
/// How far from an intercom its speaker is heard (Luna `hearers(7)`).
pub const INTERCOM_RANGE: i32 = 7;
/// How far someone talking into a radio is heard directly (Luna `message_range = 1`).
pub const RADIO_TALK_RANGE: i32 = 1;

/// The handheld radio and the wall intercom prototypes.
pub const RADIO: &str = "radio";
pub const INTERCOM: &str = "intercom";
/// The engineering headset prototype.
pub const HEADSET: &str = "headset-eng";

/// On a wall-mounted intercom: what `:i` talks into.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intercom;

/// A radio set: handheld or wall-mounted.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Radio {
    pub frequency: u16,
    /// Microphone engaged: it transmits whatever it hears nearby (Luna `broadcasting`).
    pub broadcasting: bool,
    /// Speaker engaged: it plays what it receives (Luna `listening`).
    pub listening: bool,
    /// How far its speaker carries.
    pub range: i32,
}

impl Radio {
    pub fn handheld() -> Self {
        Self {
            frequency: COMMON_FREQUENCY,
            broadcasting: false,
            listening: true,
            range: RADIO_RANGE,
        }
    }

    pub fn intercom() -> Self {
        Self {
            range: INTERCOM_RANGE,
            ..Self::handheld()
        }
    }
}

/// A frequency as BYOND shows it: 1459 → "145.9".
pub fn format_frequency(frequency: u16) -> String {
    format!("{}.{}", frequency / 10, frequency % 10)
}

/// Where speech goes besides the room (Luna's `department_radio_keys`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioMode {
    RightHand,
    LeftHand,
    Intercom,
    /// The headset on the ears (Luna `;`).
    Headset,
    /// Not into a radio at all (Luna `:w`).
    Whisper,
}

/// Split a radio prefix (`:r`, `#l`, `.i`, or the same keys on a Russian layout) off the
/// front of what was typed. The rest is trimmed.
pub fn split_mode(text: &str) -> (Option<RadioMode>, &str) {
    if let Some(rest) = text.trim_start().strip_prefix(';') {
        return (Some(RadioMode::Headset), rest.trim_start());
    }
    let mut chars = text.trim_start().chars();
    let (Some(mark), Some(key)) = (chars.next(), chars.next()) else {
        return (None, text);
    };
    if !matches!(mark, ':' | '#' | '.') {
        return (None, text);
    }
    let mode = match key.to_lowercase().next() {
        Some('r' | 'к') => RadioMode::RightHand,
        Some('l' | 'д') => RadioMode::LeftHand,
        Some('i' | 'ш') => RadioMode::Intercom,
        Some('w' | 'ц') => RadioMode::Whisper,
        _ => return (None, text),
    };
    (Some(mode), chars.as_str().trim_start())
}

pub fn spawn_intercom(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.as_str(), ""),
            Radio::intercom(),
            Intercom,
        ))
        .id()
}

/// Everything switching a radio's microphone needs.
#[derive(SystemParam)]
pub struct RadioWork<'w, 's> {
    radios: Query<'w, 's, &'static mut Radio>,
    intercoms: Query<'w, 's, (), With<Intercom>>,
}

impl RadioWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.radios.contains(target)
    }

    pub fn is_intercom(&self, target: Entity) -> bool {
        self.intercoms.contains(target)
    }

    /// An empty hand on an intercom, or a radio clicked in the active hand, engages or
    /// disengages its microphone (the "Microphone" switch on Luna's radio window).
    pub fn toggle_microphone(&mut self, target: Entity) -> Option<ClickOutcome> {
        let mut radio = self.radios.get_mut(target).ok()?;
        radio.broadcasting = !radio.broadcasting;
        Some(ClickOutcome::told(if radio.broadcasting {
            "radio-microphone-on"
        } else {
            "radio-microphone-off"
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_pick_the_radio() {
        assert_eq!(split_mode(":r Hello"), (Some(RadioMode::RightHand), "Hello"));
        assert_eq!(split_mode("#l hi"), (Some(RadioMode::LeftHand), "hi"));
        assert_eq!(split_mode(".i Help!"), (Some(RadioMode::Intercom), "Help!"));
        assert_eq!(split_mode(":ш помогите"), (Some(RadioMode::Intercom), "помогите"));
        assert_eq!(split_mode(":К привет"), (Some(RadioMode::RightHand), "привет"));
        assert_eq!(split_mode("Hello :i"), (None, "Hello :i"));
        assert_eq!(split_mode(":x what"), (None, ":x what"));
        assert_eq!(split_mode(":"), (None, ":"));
        assert_eq!(
            split_mode("; on the headset"),
            (Some(RadioMode::Headset), "on the headset")
        );
    }

    #[test]
    fn frequencies_read_like_luna() {
        assert_eq!(format_frequency(COMMON_FREQUENCY), "145.9");
        assert_eq!(format_frequency(1357), "135.7");
    }
}
