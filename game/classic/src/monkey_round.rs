//! The monkey round (Luna's `monkey` mode): soon after the start a few of the crew become monkeys,
//! and the monkeys win if one of them gets away on the pods with no human beside it. Spec:
//! `docs/reference/monkey-restructuring.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, DisplayName, GridPos, HumanLook, MobStat};

use crate::actions::Outbox;
use crate::health::Vitals;
use crate::monkeys::MONKEY;
use crate::movement::Mob;
use crate::objects::PlayerControlled;
use crate::rng::GameRng;
use crate::time::WorldTime;

pub const MODE: &str = "monkey";
/// How long after the start the crew turn (Luna's 50 ds).
pub const TURN_DS: f32 = 50.0;
/// The fewest players for anyone to turn, and the most who do.
pub const MIN_PLAYERS: usize = 3;
pub const MAX_MONKEYS: usize = 4;

/// Present only in a monkey round: when the crew turn, until they have.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct MonkeyRound {
    pub turn_at: Option<f32>,
}

/// Someone who used to be a person, and who.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Monkeyized {
    pub was: String,
}

pub fn setup_round(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    world.insert_resource(MonkeyRound {
        turn_at: Some(now + TURN_DS),
    });
}

/// How many of so many players turn (Luna: a third of the rest, and one, at most four).
pub fn how_many(players: usize) -> usize {
    if players < MIN_PLAYERS {
        return 0;
    }
    ((players - 1) / 3 + 1).min(MAX_MONKEYS)
}

pub fn run_monkey_round(world: &mut World) {
    let Some(at) = world.get_resource::<MonkeyRound>().and_then(|r| r.turn_at) else {
        return;
    };
    if world.resource::<WorldTime>().deciseconds() < at {
        return;
    }
    world.resource_mut::<MonkeyRound>().turn_at = None;
    let mut people = world.query_filtered::<Entity, (With<PlayerControlled>, With<HumanLook>)>();
    let mut players: Vec<Entity> = people.iter(world).collect();
    players.sort();
    for _ in 0..how_many(players.len()) {
        let i = world.resource_mut::<GameRng>().range(0, players.len() as i32 - 1) as usize;
        let mob = players.remove(i);
        monkeyize(world, mob);
    }
}

/// Turn someone into a monkey (Luna's `monkeyize`): all they had falls to the floor.
pub fn monkeyize(world: &mut World, mob: Entity) {
    let was = world
        .get::<DisplayName>(mob)
        .map_or_else(String::new, |n| n.0.clone());
    crate::jobs::drop_everything(world, mob);
    if let Some(mut look) = world.get_mut::<Appearance>(mob) {
        look.sprite = MONKEY.to_owned();
    }
    world
        .entity_mut(mob)
        .remove::<HumanLook>()
        .insert((DisplayName(MONKEY.to_owned()), Monkeyized { was }));
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new("monkey-you-are"));
    }
}

/// Who wears a monkey's shape: the labs' monkeys, the crew turned, a changeling in lesser form.
pub fn is_monkey(look: &Appearance) -> bool {
    look.sprite == MONKEY
}

/// Monkeys win if a live one is on the pods and no live person is (Luna `declare_completion`).
pub fn monkeys_won(world: &mut World) -> bool {
    let mut mobs =
        world.query_filtered::<(&GridPos, &Appearance, Option<&Vitals>, Has<HumanLook>), With<Mob>>();
    let aboard: Vec<(bool, bool, rs13_world::TilePos)> = mobs
        .iter(world)
        .filter(|(_, _, v, _)| v.is_none_or(|v| v.stat != MobStat::Dead))
        .map(|(p, look, _, human)| (is_monkey(look), human, p.0))
        .collect();
    let mut monkey = false;
    for (is_monkey, human, at) in aboard {
        if !(is_monkey || human) || !crate::escape::in_pod(world, at) {
            continue;
        }
        if human {
            return false;
        }
        monkey = true;
    }
    monkey
}

/// The end of a monkey round.
pub fn report(world: &mut World) -> Vec<LocMsg> {
    if world.get_resource::<MonkeyRound>().is_none() {
        return Vec::new();
    }
    if !monkeys_won(world) {
        return vec![LocMsg::new("monkey-lost")];
    }
    let mut said = vec![LocMsg::new("monkey-won")];
    let mut turned = world.query::<&Monkeyized>();
    let mut names: Vec<String> = turned.iter(world).map(|m| m.was.clone()).collect();
    names.sort();
    for name in names {
        said.push(LocMsg::new("monkey-was").arg("name", name));
    }
    said
}
