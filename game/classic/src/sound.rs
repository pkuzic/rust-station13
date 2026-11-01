//! Positional sound: what is heard, by whom, and how loud. Spec: `docs/reference/sound.md`.
//!
//! Gameplay pushes a [`SoundEvent`] into [`SoundEvents`], the way it pushes messages into
//! `combat::VisibleEvents`. At the end of each tick [`deliver_sounds`] decides, for every
//! conscious mob, whether it hears each sound and how loud, and leaves what it heard in its
//! [`Earshot`] for the server to send to the player's client. Volume falls with distance,
//! with every wall or closed door in the way, and with thin air: nothing carries through
//! vacuum except to someone on the very tile.
//!
//! Some sounds come from changes rather than from a click: [`door_sounds`],
//! [`closet_sounds`] and [`light_sounds`] watch airlocks, closets and lights and play a
//! sound when they start opening, closing, refuse, or break, whatever made them do it.

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
use rs13_simulation::atmos::{AtmosGrid, Cell};
use rs13_world::{Dir, GridPos, MobStat, TilePos};

use crate::atmos::StationAtmos;
use crate::closets::Closet;
use crate::doors::{Airlock, DoorState};
use crate::health::Vitals;
use crate::lights::{Light, LightStatus};
use crate::movement::Mob;

/// How far a sound carries at most, in tiles, before `extra range` (Luna `world.view`).
pub const SOUND_RANGE: i32 = 7;
/// Share of the volume lost at the edge of the range; it falls linearly with distance.
pub const DISTANCE_FALLOFF: f32 = 0.6;
/// Share of the volume that gets through each wall, closed airlock or closed edge.
pub const OBSTRUCTION_PASS: f32 = 0.35;
/// More obstructions than this and nothing gets through.
pub const MAX_OBSTRUCTIONS: u32 = 2;
/// Below this pressure (kPa) at either end, no sound carries through the air.
pub const MIN_SOUND_PRESSURE: f64 = 10.0;
/// Share of the volume felt on the very tile of the sound when there is no air to carry it
/// (through the floor, the tool in the hand).
pub const CONTACT_PASS: f32 = 0.5;
/// Quieter than this (out of 100) is not sent at all.
pub const MIN_VOLUME: f32 = 1.0;
/// Most sounds kept waiting in one mob's [`Earshot`].
pub const EARSHOT_LIMIT: usize = 32;

// Sound ids. The client maps each to one or more of Luna's `.ogg` files.
pub const AIRLOCK: &str = "airlock";
pub const DOOR_DENY: &str = "door-deny";
pub const CLOSET_CLICK: &str = "closet-click";
pub const GLASS_HIT: &str = "glass-hit";
pub const RATCHET: &str = "ratchet";
pub const CROWBAR: &str = "crowbar";
pub const SCREWDRIVER: &str = "screwdriver";
pub const WIRECUTTER: &str = "wirecutter";
pub const WELDER: &str = "welder";
pub const SPARKS: &str = "sparks";
pub const DISPOSAL_FLUSH: &str = "disposal-flush";
pub const HISS: &str = "hiss";
pub const WARNING_BUZZER: &str = "warning-buzzer";
/// Every sound id the game plays.
pub const ALL_SOUNDS: &[&str] = &[
    AIRLOCK,
    DOOR_DENY,
    CLOSET_CLICK,
    GLASS_HIT,
    RATCHET,
    CROWBAR,
    SCREWDRIVER,
    WIRECUTTER,
    WELDER,
    SPARKS,
    DISPOSAL_FLUSH,
    HISS,
    WARNING_BUZZER,
];

/// A sound made at a tile this tick.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundEvent {
    /// Sound id, e.g. [`AIRLOCK`].
    pub sound: &'static str,
    pub at: TilePos,
    /// Loudness at the source, 0–100, as Luna's `playsound` volume.
    pub volume: f32,
    /// Furthest a listener can be, in tiles (Chebyshev distance, as BYOND's `range`).
    pub range: i32,
    /// A mob that should not hear it.
    pub except: Option<Entity>,
}

impl SoundEvent {
    /// A sound heard up to [`SOUND_RANGE`] tiles away.
    pub fn new(sound: &'static str, at: TilePos, volume: f32) -> Self {
        Self {
            sound,
            at,
            volume,
            range: SOUND_RANGE,
            except: None,
        }
    }

    /// Carry further (or, negative, less far) than usual, as Luna's `extrarange`.
    pub fn extra_range(mut self, extra: i32) -> Self {
        self.range = (SOUND_RANGE + extra).max(0);
        self
    }
}

/// Sounds made this tick, delivered to listeners at the end of it.
#[derive(Resource, Debug, Default)]
pub struct SoundEvents(pub Vec<SoundEvent>);

impl SoundEvents {
    pub fn play(&mut self, sound: &'static str, at: TilePos, volume: f32) {
        self.0.push(SoundEvent::new(sound, at, volume));
    }

    /// The sound of using this tool, if it makes one (Luna's volumes). Welding has its own
    /// sound, played wherever a lit welding tool is used.
    pub fn tool_used(&mut self, tool: Option<&str>, at: TilePos) {
        let (sound, volume) = match tool {
            Some("wrench") => (RATCHET, 50.0),
            Some("crowbar") => (CROWBAR, 50.0),
            Some("screwdriver") => (SCREWDRIVER, 50.0),
            Some("wirecutters") => (WIRECUTTER, 100.0),
            _ => return,
        };
        self.play(sound, at, volume);
    }
}

/// Pairs of neighbouring tiles with something closed on the edge between them that stops
/// sound, such as a firedoor. Whoever owns such barriers keeps this up to date; order within
/// a pair does not matter.
#[derive(Resource, Debug, Default)]
pub struct SoundBarriers(pub HashSet<(TilePos, TilePos)>);

impl SoundBarriers {
    pub fn blocks(&self, a: TilePos, b: TilePos) -> bool {
        self.0.contains(&(a, b)) || self.0.contains(&(b, a))
    }
}

/// A sound as one mob hears it.
#[derive(Debug, Clone, PartialEq)]
pub struct HeardSound {
    pub sound: &'static str,
    /// Where it came from, for panning.
    pub at: TilePos,
    /// How loud it is for this mob, 0–100.
    pub volume: f32,
}

/// Sounds this mob heard, waiting to be sent to its player.
#[derive(Component, Debug, Default)]
pub struct Earshot(pub Vec<HeardSound>);

/// What sound travels through. Implemented over the tile map for the game, and by hand in
/// tests. Positions carry `z`, so a deck change is just another step on the path: when
/// decks connect, a floor between them becomes an obstruction like a wall.
pub trait SoundMedium {
    /// How many obstructions this tile counts as when a sound passes through it.
    fn obstruction(&self, at: TilePos) -> u32;
    /// Whether the edge between two neighbouring tiles is closed.
    fn edge_closed(&self, from: TilePos, to: TilePos) -> bool;
    /// Air pressure at a tile, kPa. `None` when no air is simulated: assume normal air.
    fn pressure(&self, at: TilePos) -> Option<f64>;
}

/// The station: walls and dense airlocks obstruct, closed edges block, and air comes from
/// the atmos solver.
pub struct StationMedium<'a> {
    /// Whether a tile is solid (walls), on whichever deck.
    pub solid: &'a dyn Fn(TilePos) -> bool,
    /// Tiles with a closed (dense) airlock on them.
    pub closed_doors: &'a HashSet<TilePos>,
    pub barriers: &'a SoundBarriers,
    pub atmos: Option<&'a AtmosGrid>,
}

impl SoundMedium for StationMedium<'_> {
    fn obstruction(&self, at: TilePos) -> u32 {
        u32::from((self.solid)(at)) + u32::from(self.closed_doors.contains(&at))
    }

    fn edge_closed(&self, from: TilePos, to: TilePos) -> bool {
        self.barriers.blocks(from, to)
    }

    fn pressure(&self, at: TilePos) -> Option<f64> {
        let atmos = self.atmos?;
        // A door tile has no air of its own; a sound there carries on the better side.
        Some(if atmos.cell(at) == Cell::Portal {
            [Dir::North, Dir::South, Dir::East, Dir::West]
                .into_iter()
                .map(|d| atmos.pressure_at(at.step(d)))
                .fold(0.0, f64::max)
        } else {
            atmos.pressure_at(at)
        })
    }
}

/// Obstructions between `from` and `to`: walls and closed doors on the straight line
/// between them (the end tiles excluded), and closed edges crossed. The line is the same
/// Bresenham line used for sight.
pub fn obstructions(medium: &impl SoundMedium, from: TilePos, to: TilePos) -> u32 {
    let path: Vec<TilePos> = crate::speech::line(from, to).collect();
    let tiles: u32 = path
        .iter()
        .filter(|p| **p != from && **p != to)
        .map(|p| medium.obstruction(*p))
        .sum();
    let edges = path.windows(2).filter(|w| medium.edge_closed(w[0], w[1])).count() as u32;
    tiles + edges
}

/// How much of a sound gets through the air between the two ends, 0–1.
fn air_pass(medium: &impl SoundMedium, from: TilePos, to: TilePos) -> f32 {
    let (Some(a), Some(b)) = (medium.pressure(from), medium.pressure(to)) else {
        return 1.0;
    };
    let air = ((a.min(b) - MIN_SOUND_PRESSURE) / (ONE_ATMOSPHERE - MIN_SOUND_PRESSURE)).clamp(0.0, 1.0);
    let air = air.sqrt() as f32;
    if from == to { air.max(CONTACT_PASS) } else { air }
}

/// How loud `sound` is for a listener at `listener`, 0–100, or `None` if it is not heard.
pub fn heard_volume(medium: &impl SoundMedium, sound: &SoundEvent, listener: TilePos) -> Option<f32> {
    let (from, to) = (sound.at, listener);
    if from.z != to.z {
        // One deck per map for now; see `SoundMedium`.
        return None;
    }
    let (dx, dy) = ((to.x - from.x) as f32, (to.y - from.y) as f32);
    if (to.x - from.x).abs() > sound.range || (to.y - from.y).abs() > sound.range {
        return None;
    }
    let blocked = obstructions(medium, from, to);
    if blocked > MAX_OBSTRUCTIONS {
        return None;
    }
    let range = sound.range.max(1) as f32;
    let distance = (dx * dx + dy * dy).sqrt().min(range);
    let volume = sound.volume
        * (1.0 - DISTANCE_FALLOFF * distance / range)
        * OBSTRUCTION_PASS.powi(blocked as i32)
        * air_pass(medium, from, to);
    (volume >= MIN_VOLUME).then_some(volume.min(100.0))
}

/// Give every conscious mob the sounds it hears this tick.
pub fn deliver_sounds(
    mut sounds: ResMut<SoundEvents>,
    barriers: Res<SoundBarriers>,
    atmos: Option<Res<StationAtmos>>,
    map: crate::decks::Decks,
    doors: Query<(&GridPos, &Airlock)>,
    mut listeners: Query<(Entity, &GridPos, &mut Earshot, Option<&Vitals>), With<Mob>>,
) {
    if sounds.0.is_empty() {
        return;
    }
    let closed_doors: HashSet<TilePos> = doors
        .iter()
        .filter(|(_, d)| d.is_dense())
        .map(|(p, _)| p.0)
        .collect();
    let medium = StationMedium {
        solid: &|p| map.is_solid(p),
        closed_doors: &closed_doors,
        barriers: &barriers,
        atmos: atmos.as_deref().map(|a| &a.0),
    };
    for sound in sounds.0.drain(..) {
        for (mob, pos, mut earshot, vitals) in &mut listeners {
            if sound.except == Some(mob) || vitals.is_some_and(|v| v.stat != MobStat::Conscious) {
                continue;
            }
            if let Some(volume) = heard_volume(&medium, &sound, pos.0) {
                // Nobody collects for a mob without a player: keep only the latest.
                if earshot.0.len() >= EARSHOT_LIMIT {
                    earshot.0.remove(0);
                }
                earshot.0.push(HeardSound {
                    sound: sound.sound,
                    at: sound.at,
                    volume,
                });
            }
        }
    }
}

/// What a door was doing at the end of the last tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoorWas {
    opening: bool,
    closing: bool,
    deny_until: Option<f32>,
}

/// Airlocks hiss when they start opening or closing, and buzz when they refuse (Luna plays
/// `airlock.ogg` at 30 on open and close; the buzz is ours, Luna only flashes the light).
pub fn door_sounds(
    mut was: Local<HashMap<Entity, DoorWas>>,
    mut sounds: ResMut<SoundEvents>,
    doors: Query<(Entity, &GridPos, &Airlock)>,
) {
    for (entity, pos, door) in &doors {
        let now = DoorWas {
            opening: matches!(door.state, DoorState::Opening { .. }),
            closing: matches!(door.state, DoorState::Closing { .. }),
            deny_until: door.deny_until,
        };
        if let Some(before) = was.insert(entity, now) {
            if (now.opening && !before.opening) || (now.closing && !before.closing) {
                sounds.play(AIRLOCK, pos.0, 30.0);
            }
            if now.deny_until.is_some() && now.deny_until != before.deny_until {
                sounds.play(DOOR_DENY, pos.0, 30.0);
            }
        }
    }
    was.retain(|e, _| doors.contains(*e));
}

/// Closets click when they open or close (Luna `click.ogg` at 15, three tiles closer).
pub fn closet_sounds(
    mut was: Local<HashMap<Entity, bool>>,
    mut sounds: ResMut<SoundEvents>,
    closets: Query<(Entity, &GridPos, &Closet)>,
) {
    for (entity, pos, closet) in &closets {
        if was
            .insert(entity, closet.opened)
            .is_some_and(|before| before != closet.opened)
        {
            sounds
                .0
                .push(SoundEvent::new(CLOSET_CLICK, pos.0, 15.0).extra_range(-3));
        }
    }
    was.retain(|e, _| closets.contains(*e));
}

/// A tube or bulb breaking makes the sound of glass (Luna `Glasshit.ogg` at 75).
pub fn light_sounds(
    mut was: Local<HashMap<Entity, LightStatus>>,
    mut sounds: ResMut<SoundEvents>,
    lights: Query<(Entity, &GridPos, &Light)>,
) {
    for (entity, pos, light) in &lights {
        let before = was.insert(entity, light.status);
        if light.status == LightStatus::Broken
            && matches!(before, Some(LightStatus::Ok | LightStatus::Burned))
        {
            sounds.play(GLASS_HIT, pos.0, 75.0);
        }
    }
    was.retain(|e, _| lights.contains(*e));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An open room with full air, and walls wherever listed.
    struct Room(Vec<TilePos>);

    impl SoundMedium for Room {
        fn obstruction(&self, at: TilePos) -> u32 {
            u32::from(self.0.contains(&at))
        }
        fn edge_closed(&self, _: TilePos, _: TilePos) -> bool {
            false
        }
        fn pressure(&self, _: TilePos) -> Option<f64> {
            None
        }
    }

    #[test]
    fn volume_falls_with_distance_and_stops_at_the_range() {
        let room = Room(Vec::new());
        let sound = SoundEvent::new(AIRLOCK, TilePos::new(0, 0, 0), 100.0);
        let here = heard_volume(&room, &sound, TilePos::new(0, 0, 0)).unwrap();
        let near = heard_volume(&room, &sound, TilePos::new(2, 0, 0)).unwrap();
        let far = heard_volume(&room, &sound, TilePos::new(7, 0, 0)).unwrap();
        assert_eq!(here, 100.0);
        assert!(near > far);
        assert!((far - 40.0).abs() < 1e-3);
        assert_eq!(heard_volume(&room, &sound, TilePos::new(8, 0, 0)), None);
        assert_eq!(heard_volume(&room, &sound, TilePos::new(0, 0, 1)), None);
    }

    /// Open air at one pressure everywhere.
    struct Air(f64);

    impl SoundMedium for Air {
        fn obstruction(&self, _: TilePos) -> u32 {
            0
        }
        fn edge_closed(&self, _: TilePos, _: TilePos) -> bool {
            false
        }
        fn pressure(&self, _: TilePos) -> Option<f64> {
            Some(self.0)
        }
    }

    #[test]
    fn thin_air_carries_less_and_vacuum_nothing() {
        let sound = SoundEvent::new(AIRLOCK, TilePos::new(0, 0, 0), 100.0);
        let to = TilePos::new(1, 0, 0);
        let full = heard_volume(&Air(ONE_ATMOSPHERE), &sound, to).unwrap();
        let thin = heard_volume(&Air(ONE_ATMOSPHERE / 2.0), &sound, to).unwrap();
        assert!(thin < full && thin > 0.5 * full);
        assert_eq!(heard_volume(&Air(MIN_SOUND_PRESSURE), &sound, to), None);
        // On the very tile, a sound is still felt.
        let felt = heard_volume(&Air(0.0), &sound, sound.at).unwrap();
        assert!((felt - 100.0 * CONTACT_PASS).abs() < 1e-3);
    }

    #[test]
    fn extra_range_shrinks_and_grows_the_range() {
        assert_eq!(
            SoundEvent::new(AIRLOCK, TilePos::new(0, 0, 0), 1.0)
                .extra_range(-3)
                .range,
            4
        );
        assert_eq!(
            SoundEvent::new(AIRLOCK, TilePos::new(0, 0, 0), 1.0)
                .extra_range(-20)
                .range,
            0
        );
    }

    #[test]
    fn each_wall_cuts_the_volume_and_three_block_it() {
        let sound = SoundEvent::new(AIRLOCK, TilePos::new(0, 0, 0), 100.0);
        let to = TilePos::new(4, 0, 0);
        let open = heard_volume(&Room(Vec::new()), &sound, to).unwrap();
        let one = heard_volume(&Room(vec![TilePos::new(1, 0, 0)]), &sound, to).unwrap();
        assert!((one - open * OBSTRUCTION_PASS).abs() < 1e-3);
        let three = Room(vec![
            TilePos::new(1, 0, 0),
            TilePos::new(2, 0, 0),
            TilePos::new(3, 0, 0),
        ]);
        assert_eq!(heard_volume(&three, &sound, to), None);
        // The ends never obstruct: a sound in a doorway is not muffled by the door.
        let ends = Room(vec![TilePos::new(0, 0, 0), to]);
        assert_eq!(heard_volume(&ends, &sound, to), Some(open));
    }
}
