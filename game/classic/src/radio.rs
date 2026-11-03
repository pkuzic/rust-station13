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
/// Department channels: their frequencies (Luna's `radiochannels`).
pub const SCIENCE: u16 = 1351;
pub const COMMAND: u16 = 1353;
pub const MEDICAL: u16 = 1355;
pub const ENGINEERING: u16 = 1357;
pub const SECURITY: u16 = 1359;
pub const SYNDICATE: u16 = 1213;
pub const SUPPLY: u16 = 1347;
/// Each channel's frequency, name in messages, and key after `:`, on both layouts.
pub const CHANNELS: [(u16, &str, char, char); 7] = [
    (SCIENCE, "science", 'n', 'т'),
    (COMMAND, "command", 'c', 'с'),
    (MEDICAL, "medical", 'm', 'ь'),
    (ENGINEERING, "engineering", 'e', 'у'),
    (SECURITY, "security", 's', 'ы'),
    (SYNDICATE, "syndicate", 't', 'е'),
    (SUPPLY, "supply", 'q', 'й'),
];

/// The channels a headset's keys give it, the first being `:h`, and whether it's listening to each
/// (Luna's `encryptionkey`s). `None` if it isn't a headset. The engineer's is as before.
pub fn headset_channels(proto: &str) -> Option<&'static [(u16, bool)]> {
    Some(match proto {
        "headset-eng" => &[(ENGINEERING, true)],
        "headset" => &[],
        "headset-sec" => &[(SECURITY, true)],
        "headset-rob" => &[(ENGINEERING, true), (SCIENCE, true), (MEDICAL, true)],
        "headset-med" => &[(MEDICAL, true)],
        "headset-sci" => &[(SCIENCE, true)],
        "headset-medsci" => &[(MEDICAL, true), (SCIENCE, true)],
        "headset-com" => &[(COMMAND, true)],
        "headset-captain" => &[
            (COMMAND, true),
            (SECURITY, true),
            (ENGINEERING, false),
            (SCIENCE, false),
            (MEDICAL, false),
            (SUPPLY, false),
        ],
        "headset-rd" => &[(SCIENCE, true), (COMMAND, true)],
        "headset-hos" => &[(SECURITY, true), (COMMAND, true)],
        "headset-ce" => &[(ENGINEERING, true), (COMMAND, true)],
        "headset-cmo" => &[(MEDICAL, true), (COMMAND, true)],
        "headset-hop" => &[(SUPPLY, true), (COMMAND, true), (SECURITY, false)],
        "headset-mine" => &[(SUPPLY, true)],
        "headset-cargo" => &[(SUPPLY, true)],
        "headset-syndicate" => &[(SYNDICATE, true)],
        _ => return None,
    })
}

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
    /// Department channels, with whether it's listening to each: a headset's.
    pub channels: &'static [(u16, bool)],
}

impl Radio {
    pub fn handheld() -> Self {
        Self {
            frequency: COMMON_FREQUENCY,
            broadcasting: false,
            listening: true,
            range: RADIO_RANGE,
            channels: &[],
        }
    }

    pub fn headset(channels: &'static [(u16, bool)]) -> Self {
        Self {
            channels,
            ..Self::handheld()
        }
    }

    /// Whether it plays what comes in on this frequency.
    pub fn hears(&self, frequency: u16) -> bool {
        self.listening
            && (self.frequency == frequency || self.channels.iter().any(|(f, on)| *f == frequency && *on))
    }

    /// The frequency for talking on this channel, or its first if none is named (Luna `:h`).
    pub fn channel(&self, wanted: Option<u16>) -> Option<u16> {
        match wanted {
            Some(wanted) => self.channels.iter().map(|(f, _)| *f).find(|f| *f == wanted),
            None => self.channels.first().map(|(f, _)| *f),
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
    /// A department channel on the headset, or its first one (Luna `:h`, `:e`, and so on).
    Channel(Option<u16>),
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
        Some('h' | 'р') => RadioMode::Channel(None),
        Some(key) => match CHANNELS.iter().find(|(.., en, ru)| key == *en || key == *ru) {
            Some((frequency, ..)) => RadioMode::Channel(Some(*frequency)),
            None => return (None, text),
        },
        None => return (None, text),
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
