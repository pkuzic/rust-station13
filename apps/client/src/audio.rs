//! Playing the sounds the server says we hear, panned left or right by where they came
//! from. The server has already worked out how loud each one is for us (distance, walls,
//! doors, air); see `docs/reference/sound.md`.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::audio::{AddAudioSource, AudioSource, Decodable, PlaybackSettings, Source, Volume};
use bevy::prelude::*;
use rodio::source::ChannelVolume;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{PlaySound, Player};
use rs13_world::GridPos;

/// A sound fully to one side this many tiles away (Luna: `dx / 8`).
const FULL_PAN_TILES: f32 = 8.0;

macro_rules! ogg {
    ($file:literal) => {
        include_bytes!(concat!("../../../client-assets/sounds/", $file)) as &[u8]
    };
}

/// Sound id → the files it plays, one picked in turn. Embedded until the server's asset
/// manifest delivers them. Sources: `client-assets/sounds/ATTRIBUTION.md`.
const SOUNDS: &[(&str, &[&[u8]])] = &[
    ("airlock", &[ogg!("machines/airlock.ogg")]),
    ("door-deny", &[ogg!("machines/buzz-two.ogg")]),
    ("closet-click", &[ogg!("machines/click.ogg")]),
    ("glass-hit", &[ogg!("effects/Glasshit.ogg")]),
    ("ratchet", &[ogg!("items/Ratchet.ogg")]),
    ("crowbar", &[ogg!("items/Crowbar.ogg")]),
    ("screwdriver", &[ogg!("items/Screwdriver.ogg")]),
    ("wirecutter", &[ogg!("items/Wirecutter.ogg")]),
    ("welder", &[ogg!("items/Welder.ogg"), ogg!("items/Welder2.ogg")]),
    ("disposal-flush", &[ogg!("machines/disposalflush.ogg")]),
    ("hiss", &[ogg!("machines/hiss.ogg")]),
    ("warning-buzzer", &[ogg!("machines/warning-buzzer.ogg")]),
    (
        "sparks",
        &[
            ogg!("effects/sparks1.ogg"),
            ogg!("effects/sparks2.ogg"),
            ogg!("effects/sparks3.ogg"),
            ogg!("effects/sparks4.ogg"),
        ],
    ),
];

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<PannedSound>()
            .init_resource::<SoundBank>()
            .add_systems(Update, play_sounds);
    }
}

/// The decoded-on-demand clips for every sound id.
#[derive(Resource)]
struct SoundBank {
    clips: HashMap<&'static str, Vec<AudioSource>>,
    /// Which variant to play next.
    turn: usize,
}

impl Default for SoundBank {
    fn default() -> Self {
        let clips = SOUNDS
            .iter()
            .map(|(id, files)| {
                let sources = files
                    .iter()
                    .map(|bytes| AudioSource {
                        bytes: Arc::from(*bytes),
                    })
                    .collect();
                (*id, sources)
            })
            .collect();
        Self { clips, turn: 0 }
    }
}

/// One playing of a clip, with its own left and right volumes.
#[derive(Asset, TypePath)]
struct PannedSound {
    clip: AudioSource,
    /// Left and right gain, 0–1.
    gains: [f32; 2],
}

impl Decodable for PannedSound {
    type Decoder = ChannelVolume<<AudioSource as Decodable>::Decoder>;

    fn decoder(&self) -> Self::Decoder {
        let inner = self.clip.decoder();
        // `ChannelVolume` sums the input channels into one without averaging them.
        let channels = f32::from(inner.channels().get());
        ChannelVolume::new(inner, self.gains.iter().map(|g| g / channels).collect())
    }
}

/// Left and right gain for a sound `dx` tiles to the right (negative: left). The near
/// side stays at full volume; the far side fades out by [`FULL_PAN_TILES`].
fn pan_gains(dx: i32) -> [f32; 2] {
    let pan = (dx as f32 / FULL_PAN_TILES).clamp(-1.0, 1.0);
    [(1.0 - pan).min(1.0), (1.0 + pan).min(1.0)]
}

fn play_sounds(
    mut bank: ResMut<SoundBank>,
    mut assets: ResMut<Assets<PannedSound>>,
    mut receivers: Query<&mut MessageReceiver<PlaySound>, With<Client>>,
    own: Query<&GridPos, (With<Player>, With<Controlled>)>,
    mut commands: Commands,
) {
    let me = own.iter().next().map(|p| p.0);
    for mut receiver in &mut receivers {
        for sound in receiver.receive() {
            let Some(clips) = bank.clips.get(sound.sound.as_str()).filter(|c| !c.is_empty()) else {
                debug!(sound = sound.sound, "unknown sound");
                continue;
            };
            let clip = clips[bank.turn % clips.len()].clone();
            bank.turn = bank.turn.wrapping_add(1);
            let dx = me.map_or(0, |me| sound.at.x - me.x);
            let handle = assets.add(PannedSound {
                clip,
                gains: pan_gains(dx),
            });
            commands.spawn((
                AudioPlayer(handle),
                PlaybackSettings {
                    volume: Volume::Linear(f32::from(sound.volume) / 100.0),
                    ..PlaybackSettings::DESPAWN
                },
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_pan_towards_their_side() {
        assert_eq!(pan_gains(0), [1.0, 1.0]);
        let [left, right] = pan_gains(4);
        assert_eq!(right, 1.0);
        assert!((left - 0.5).abs() < 1e-6);
        assert_eq!(pan_gains(-20), [1.0, 0.0]);
    }

    #[test]
    fn every_sound_has_a_clip() {
        for (id, files) in SOUNDS {
            assert!(!files.is_empty(), "{id}");
            assert!(files.iter().all(|f| f.starts_with(b"OggS")), "{id}");
        }
        for id in rs13_classic::sound::ALL_SOUNDS {
            assert!(SOUNDS.iter().any(|(known, _)| known == id), "no clip for {id}");
        }
    }
}
