//! Conveyor belts and their switches: a switch runs every belt with its id forward or
//! backward, and running belts carry what lies on them a tile every machine cycle. Spec:
//! `docs/reference/conveyors.md`.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, Dense, Dir, GridPos, NetId, ObjectSpec, TilePos};

use crate::machines::MACHINE_TICKS;
use crate::movement::Mob;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;

pub const CONVEYOR: &str = "conveyor";
pub const CONVEYOR_SWITCH: &str = "conveyor-switch";
/// Power a running belt draws, W (Luna `use_power(100)`).
pub const BELT_WATTS: f64 = 100.0;
/// Most things one belt moves in a cycle (Luna `items_moved >= 10`).
pub const MOVES_PER_BELT: usize = 10;

/// A conveyor belt: the way it carries things running forward and backward (a corner
/// turns them), the switches it answers to, and how it runs: 1 forward, -1 backward, 0
/// stopped.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Conveyor {
    pub dir: Dir,
    pub forwards: Dir,
    pub backwards: Dir,
    pub id: String,
    pub operating: i8,
}

impl Conveyor {
    /// Which way it carries things now, if it runs.
    pub fn moving(&self) -> Option<Dir> {
        match self.operating {
            1 => Some(self.forwards),
            -1 => Some(self.backwards),
            _ => None,
        }
    }
}

/// A conveyor switch: off (0), forward (1) or reverse (-1); where it was before it was
/// last turned off; whether it only runs one way; and whether it was just thrown, for the
/// belts to follow on the next cycle.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct ConveyorSwitch {
    pub id: String,
    pub position: i8,
    pub last_pos: i8,
    pub oneway: bool,
    pub operated: bool,
}

impl ConveyorSwitch {
    /// Luna `conveyor_switch/attack_hand`: off goes the other way to the one it last ran
    /// (forward the first time); on goes off. A one-way switch goes forward and off.
    pub fn throw(&mut self) {
        if self.oneway {
            self.position = if self.position == 0 { 1 } else { 0 };
        } else if self.position == 0 {
            self.position = if self.last_pos < 0 { 1 } else { -1 };
            self.last_pos = 0;
        } else {
            self.last_pos = self.position;
            self.position = 0;
        }
        self.operated = true;
    }

    fn state(&self) -> &'static str {
        match self.position {
            1 => "switch-fwd",
            -1 => "switch-rev",
            _ => "switch-off",
        }
    }
}

/// Luna `conveyor/New`: the map's facing gives the forward and backward ways; a corner
/// (a diagonal facing) carries things round it.
pub fn belt_ways(dir: Dir) -> (Dir, Dir) {
    match dir {
        Dir::North => (Dir::North, Dir::South),
        Dir::East => (Dir::East, Dir::West),
        Dir::West => (Dir::West, Dir::East),
        Dir::NorthEast => (Dir::East, Dir::South),
        Dir::NorthWest => (Dir::South, Dir::West),
        Dir::SouthEast => (Dir::North, Dir::East),
        Dir::SouthWest => (Dir::West, Dir::North),
        Dir::South => (Dir::South, Dir::North),
    }
}

fn map_id(spec: &ObjectSpec) -> String {
    spec.vars
        .get("id")
        .map(|v| v.trim().trim_matches('"').to_owned())
        .unwrap_or_default()
}

pub fn spawn_conveyor(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let dir = spec.dir.unwrap_or(Dir::South);
    let (forwards, backwards) = belt_ways(dir);
    // Luna `centcom_auto` runs from the start.
    let on = spec.vars.get("is_on").is_some_and(|v| v.trim() == "1");
    let belt = Conveyor {
        dir,
        forwards,
        backwards,
        id: map_id(spec),
        operating: i8::from(on),
    };
    let look = belt_look(&belt);
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(CONVEYOR, look),
            belt,
            PowerUser::new(Channel::Equipment, 0.0),
        ))
        .id()
}

pub fn spawn_switch(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let switch = ConveyorSwitch {
        id: map_id(spec),
        position: 0,
        last_pos: -1,
        oneway: spec.vars.get("oneway").is_some_and(|v| v.trim() == "1"),
        operated: false,
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(CONVEYOR_SWITCH, switch.state()),
            switch,
        ))
        .id()
}

fn dir_name(dir: Dir) -> &'static str {
    match dir {
        Dir::North => "north",
        Dir::South => "south",
        Dir::East => "east",
        Dir::West => "west",
        Dir::NorthEast => "northeast",
        Dir::NorthWest => "northwest",
        Dir::SouthEast => "southeast",
        Dir::SouthWest => "southwest",
    }
}

/// Luna `conveyor/update`: `conveyor0`, `conveyor1` or `conveyor-1`, in its facing.
fn belt_look(belt: &Conveyor) -> String {
    format!("conveyor{}-{}", belt.operating, dir_name(belt.dir))
}

/// Throwing a switch sets every switch with its id the same way (Luna `attack_hand`).
pub fn throw_switch(target: Entity, switches: &mut Query<&mut ConveyorSwitch>) -> bool {
    let Ok(mut switch) = switches.get_mut(target) else {
        return false;
    };
    switch.throw();
    let (id, position) = (switch.id.clone(), switch.position);
    for mut other in switches.iter_mut() {
        if other.id == id {
            other.position = position;
        }
    }
    true
}

/// Each machine cycle: belts follow the switches thrown since the last one, and running,
/// powered belts carry up to ten loose things a tile on (Luna `conveyor_switch/process`,
/// `conveyor/process`). What moves is decided before anything moves, so nothing rides two
/// belts in one cycle.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn run_conveyors(
    time: Res<WorldTime>,
    map: crate::decks::Decks,
    mut switches: Query<&mut ConveyorSwitch>,
    mut belts: Query<(&GridPos, &mut Conveyor, &mut PowerUser)>,
    things: Query<(Entity, &GridPos), (Without<Conveyor>, Without<crate::closets::InCloset>)>,
    pullable: crate::pulling::Pullable,
    blockers: Query<&GridPos, (With<Dense>, Without<Mob>)>,
    mobs: Query<&GridPos, With<Mob>>,
    firedoors: Query<(&GridPos, &crate::firedoors::Firedoor), Without<Mob>>,
    barriers: crate::windoors::EdgeBarriers,
    mut commands: Commands,
) {
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    for mut switch in &mut switches {
        if !switch.operated {
            continue;
        }
        switch.operated = false;
        for (_, mut belt, _) in &mut belts {
            if belt.id == switch.id {
                belt.operating = switch.position;
            }
        }
    }
    let mut sealed = crate::firedoors::closed_edges(firedoors.iter());
    sealed.extend(barriers.iter());
    let blocked: HashSet<TilePos> = blockers.iter().map(|p| p.0).collect();
    let mut mob_tiles: HashSet<TilePos> = mobs.iter().map(|p| p.0).collect();
    let mut moves: Vec<(Entity, TilePos, TilePos, bool)> = Vec::new();
    for (at, belt, mut power) in &mut belts {
        let watts = if belt.operating != 0 { BELT_WATTS } else { 0.0 };
        if power.watts != watts {
            power.watts = watts;
        }
        let Some(dir) = belt.moving().filter(|_| power.powered) else {
            continue;
        };
        let to = at.0.step(dir);
        let mut riders: Vec<Entity> = things
            .iter()
            .filter(|(e, p)| p.0 == at.0 && pullable.check(*e))
            .map(|(e, _)| e)
            .collect();
        riders.sort();
        for rider in riders.into_iter().take(MOVES_PER_BELT) {
            moves.push((rider, at.0, to, mobs.contains(rider)));
        }
    }
    for (rider, from, to, is_mob) in moves {
        // Luna `step`: walls, dense things, shut edges and people stop it.
        if map.is_solid(to)
            || blocked.contains(&to)
            || mob_tiles.contains(&to)
            || crate::firedoors::crosses_closed_edge(&sealed, from, to)
        {
            continue;
        }
        if is_mob {
            mob_tiles.remove(&from);
            mob_tiles.insert(to);
        }
        commands.entity(rider).insert(GridPos(to));
    }
}

/// Sprites follow the belts and switches.
pub fn update_conveyor_looks(
    mut belts: Query<(&Conveyor, &PowerUser, &mut Appearance), Without<ConveyorSwitch>>,
    mut switches: Query<(&ConveyorSwitch, &mut Appearance), Changed<ConveyorSwitch>>,
) {
    for (belt, power, mut look) in &mut belts {
        // Luna `update`: an unpowered belt shows stopped.
        let shown = Conveyor {
            operating: if power.powered { belt.operating } else { 0 },
            ..belt.clone()
        };
        let state = belt_look(&shown);
        if look.state != state {
            look.state = state;
        }
    }
    for (switch, mut look) in &mut switches {
        if look.state != switch.state() {
            look.state = switch.state().to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn switch(oneway: bool) -> ConveyorSwitch {
        ConveyorSwitch {
            id: String::new(),
            position: 0,
            last_pos: -1,
            oneway,
            operated: false,
        }
    }

    #[test]
    fn a_switch_goes_reverse_off_forward_off() {
        let mut s = switch(false);
        let mut seen = Vec::new();
        for _ in 0..4 {
            s.throw();
            seen.push(s.position);
        }
        assert_eq!(seen, [1, 0, -1, 0]);
    }

    #[test]
    fn a_one_way_switch_goes_forward_and_off() {
        let mut s = switch(true);
        s.throw();
        assert_eq!(s.position, 1);
        s.throw();
        assert_eq!(s.position, 0);
    }
}
