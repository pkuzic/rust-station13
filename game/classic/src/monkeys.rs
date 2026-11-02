//! Monkeys (Luna's `carbon/monkey`): animals that wander about the labs. They have bodies like
//! anyone's, so they can be hit, pulled and laid in a morgue, but no player. Spec:
//! `docs/reference/monkeys.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Dir, DisplayName, GridPos, MobStat};

use crate::actions::Outbox;
use crate::combat::VisibleEvents;
use crate::health::Vitals;
use crate::machines::MACHINE_TICKS;
use crate::movement::MoveIntent;
use crate::rng::GameRng;
use crate::time::WorldTime;

pub const MONKEY: &str = "monkey";
/// Chance in percent each life cycle of a step, and of doing something monkeyish (Luna `Life`).
pub const STEP_CHANCE: f32 = 33.0;
pub const EMOTE_CHANCE: f32 = 1.0;

/// Marks a monkey.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Monkey;

/// What a monkey does now and then (Luna `emote`).
const EMOTES: [&str; 4] = ["scratch", "jump", "roll", "tail"];

/// Every 2 s a monkey awake may take a random step or put on a show.
pub fn run_monkeys(
    time: Res<WorldTime>,
    mut rng: ResMut<GameRng>,
    mut visible: ResMut<VisibleEvents>,
    mut monkeys: Query<(&GridPos, &DisplayName, &Vitals, &mut MoveIntent, &mut Outbox), With<Monkey>>,
) {
    let cycle = time.tick.0.is_multiple_of(MACHINE_TICKS);
    for (pos, name, vitals, mut intent, mut outbox) in &mut monkeys {
        // Nobody reads what it hears.
        outbox.0.clear();
        // One step at a time.
        if intent.dir.is_some() {
            intent.dir = None;
        }
        if !cycle || vitals.stat != MobStat::Conscious {
            continue;
        }
        if rng.prob(STEP_CHANCE) {
            intent.dir = Some(match rng.range(1, 4) {
                1 => Dir::North,
                2 => Dir::South,
                3 => Dir::East,
                _ => Dir::West,
            });
        }
        if rng.prob(EMOTE_CHANCE) {
            let act = EMOTES[(rng.range(1, 4) - 1) as usize];
            visible.0.push((pos.0, emote(&name.0, act), None));
        }
    }
}

fn emote(name: &str, act: &str) -> LocMsg {
    LocMsg::new("monkey-emote")
        .arg("name", name.to_owned())
        .arg("act", act.to_owned())
}

/// What a monkey with this id is called (Luna `monkey (123)`).
pub fn monkey_name(id: u64) -> String {
    format!("monkey ({id})")
}
