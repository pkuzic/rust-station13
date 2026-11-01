//! Blast doors (Luna's `poddoor`, "Podlock") and the wall buttons that work them (Luna's
//! `door_control`): a heavy shutter that seals a tile against people, sight, light and
//! air, opened and closed only from a button with its id. Burn chambers and the
//! incinerator vent to space through them. Spec: `docs/reference/blast-doors.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_world::{Appearance, Dense, GridPos, NetId, ObjectSpec, Opaque, TilePos};

use crate::actions::ClickOutcome;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;

/// The blast door prototype.
pub const BLAST_DOOR: &str = "blast-door";
/// The wall button prototype.
pub const DOOR_BUTTON: &str = "door-button";
/// How long a blast door takes to open before it stops blocking, ds (Luna `spawn(10)`).
pub const OPENING_DS: f32 = 10.0;
/// How long its closing animation shows, ds; it blocks at once (Luna `close`).
pub const CLOSING_DS: f32 = 10.0;
/// How long a pressed button shows pressed, ds (Luna `sleep(10)`).
pub const BUTTON_DOWN_DS: f32 = 10.0;

/// Where a blast door is in its cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shutter {
    Closed,
    /// Opening until this time, ds; still blocks.
    Opening(f32),
    Open,
    /// Closing until this time, ds; blocks already.
    Closing(f32),
}

/// A blast door, and the id its buttons share.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct BlastDoor {
    pub id: String,
    pub shutter: Shutter,
}

impl BlastDoor {
    /// Whether it blocks people, sight, light and air.
    pub fn blocks(&self) -> bool {
        !matches!(self.shutter, Shutter::Open)
    }

    /// Luna's `pdoor1` shut, `pdoorc0` opening, `pdoor0` open, `pdoorc1` closing.
    pub fn look(&self) -> &'static str {
        match self.shutter {
            Shutter::Closed => "pdoor1",
            Shutter::Opening(_) => "pdoorc0",
            Shutter::Open => "pdoor0",
            Shutter::Closing(_) => "pdoorc1",
        }
    }
}

/// A wall button for the blast doors with its id.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct DoorButton {
    pub id: String,
    /// Flips on each press (Luna `toggled`); shows in its sprite.
    pub toggled: bool,
    /// While pressed: when it comes back up, ds.
    pub down_until: Option<f32>,
}

fn var(spec: &ObjectSpec, name: &str) -> Option<String> {
    spec.vars.get(name).map(|v| v.trim().trim_matches('"').to_owned())
}

/// A blast door, shut unless the map says `density = 0`.
pub fn spawn_blast_door(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let open = var(spec, "density").as_deref() == Some("0");
    let door = BlastDoor {
        id: var(spec, "id").unwrap_or_default(),
        shutter: if open { Shutter::Open } else { Shutter::Closed },
    };
    let mut entity = world.spawn((
        GridPos(pos),
        id,
        Appearance::new(BLAST_DOOR, door.look()),
        PowerUser::new(Channel::Equipment, 0.0),
    ));
    if door.blocks() {
        entity.insert((Dense, Opaque));
    }
    entity.insert(door).id()
}

pub fn spawn_door_button(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(DOOR_BUTTON, "doorctrl00"),
            DoorButton {
                id: var(spec, "id").unwrap_or_default(),
                toggled: false,
                down_until: None,
            },
            PowerUser::new(Channel::Equipment, 0.0),
        ))
        .id()
}

/// Everything clicking blast doors and their buttons needs.
#[derive(SystemParam)]
pub struct BlastDoorWork<'w, 's> {
    doors: Query<'w, 's, (&'static mut BlastDoor, &'static PowerUser)>,
    buttons: Query<'w, 's, (&'static mut DoorButton, &'static PowerUser), Without<BlastDoor>>,
}

impl BlastDoorWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.doors.contains(target) || self.buttons.contains(target)
    }

    /// Luna `door_control/attack_hand`: a powered button opens every shut blast door with
    /// its id and shuts every open one; anything in the hand works as a hand. Luna
    /// `poddoor/attackby`: a crowbar pries a shut blast door open, but only without power.
    /// Nobody is told anything. The caller checks reach.
    pub fn use_on(&mut self, target: Entity, held: Option<&str>, now: f32) -> Option<ClickOutcome> {
        if let Ok((mut door, power)) = self.doors.get_mut(target) {
            if held == Some("crowbar") && !power.powered && door.shutter == Shutter::Closed {
                door.shutter = Shutter::Opening(now + OPENING_DS);
            }
            return Some(ClickOutcome::default());
        }
        let (mut button, power) = self.buttons.get_mut(target).ok()?;
        if !power.powered {
            return Some(ClickOutcome::default());
        }
        button.toggled = !button.toggled;
        button.down_until = Some(now + BUTTON_DOWN_DS);
        let id = button.id.clone();
        for (mut door, _) in &mut self.doors {
            if door.id != id {
                continue;
            }
            match door.shutter {
                Shutter::Closed => door.shutter = Shutter::Opening(now + OPENING_DS),
                Shutter::Open => door.shutter = Shutter::Closing(now + CLOSING_DS),
                // Luna ignores a door already moving.
                Shutter::Opening(_) | Shutter::Closing(_) => {}
            }
        }
        Some(ClickOutcome::default())
    }
}

/// Finish opening and closing, keep each door's blocking and look in step with where it
/// is, and let pressed buttons back up.
#[allow(clippy::type_complexity)]
pub fn run_blast_doors(
    time: Res<WorldTime>,
    mut commands: Commands,
    mut doors: Query<(Entity, &mut BlastDoor, &mut Appearance, Has<Dense>), Without<DoorButton>>,
    mut buttons: Query<(&mut DoorButton, &PowerUser, &mut Appearance), Without<BlastDoor>>,
) {
    let now = time.deciseconds();
    for (entity, mut door, mut look, dense) in &mut doors {
        match door.shutter {
            Shutter::Opening(until) if now >= until => door.shutter = Shutter::Open,
            Shutter::Closing(until) if now >= until => door.shutter = Shutter::Closed,
            _ => {}
        }
        if door.blocks() && !dense {
            commands.entity(entity).insert((Dense, Opaque));
        } else if !door.blocks() && dense {
            commands.entity(entity).remove::<(Dense, Opaque)>();
        }
        let state = door.look();
        if look.state != state {
            look.state = state.to_owned();
        }
    }
    for (mut button, power, mut look) in &mut buttons {
        if button.down_until.is_some_and(|t| now >= t) {
            button.down_until = None;
        }
        // Luna `doorctrl-p` without power, `doorctrl1` while pressed, else `doorctrl0`
        // and whether it was last toggled on.
        let state = if !power.powered {
            "doorctrl-p".to_owned()
        } else if button.down_until.is_some() {
            "doorctrl1".to_owned()
        } else {
            format!("doorctrl0{}", u8::from(button.toggled))
        };
        if look.state != state {
            look.state = state;
        }
    }
}
