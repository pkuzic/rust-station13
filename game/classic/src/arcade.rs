//! Arcade machines (Luna's `computer/arcade`): a turn-based fight against a space villain, with
//! a toy for winning. Spec: `docs/reference/arcade.md`.

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, ArcadePanelView, Dense, GridPos, MobStat, NetId, TilePos};

use crate::actions::in_reach;
use crate::health::Vitals;
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const ARCADE: &str = "arcade";
/// How long the villain takes to answer, ds (Luna `sleep(10)`).
pub const ANSWER_DS: f32 = 10.0;
/// The toys it hands out (Luna's `toy/prize` kinds we have sprites for).
pub const PRIZES: [&str; 3] = ["toy-ripley", "toy-gygax", "toy-durand"];

const TITLES: [&str; 7] = [
    "automatic",
    "farmer",
    "lord",
    "professor",
    "evil",
    "dread-king",
    "space",
];
const NAMES: [&str; 8] = [
    "melonoid",
    "murdertron",
    "sorcerer",
    "ruin",
    "jeff",
    "ectoplasm",
    "crushulon",
    "coder",
];

/// A move in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcadeOp {
    Attack,
    Heal,
    Recharge,
    NewGame,
}

/// What a mob asked of an arcade machine this tick.
#[derive(Component, Debug, Default)]
pub struct PendingArcadeOps(pub Vec<(Entity, ArcadeOp)>);

/// What a mob's open arcade window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct ArcadePanelOut(pub Option<ArcadePanelView>);

/// A game in progress (Luna's vars of the same names).
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Arcade {
    pub title: &'static str,
    pub name: &'static str,
    /// What just happened: a message id, and its numbers.
    pub message: &'static str,
    pub value: i32,
    pub cost: i32,
    pub player_hp: i32,
    pub player_mp: i32,
    pub enemy_hp: i32,
    pub enemy_mp: i32,
    pub game_over: bool,
    /// The player has moved and the villain answers at this time, ds.
    pub answer_at: Option<f32>,
}

impl Arcade {
    fn new(title: &'static str, name: &'static str) -> Self {
        Self {
            title,
            name,
            message: "arcade-new",
            value: 0,
            cost: 0,
            player_hp: 30,
            player_mp: 10,
            enemy_hp: 45,
            enemy_mp: 20,
            game_over: false,
            answer_at: None,
        }
    }

    /// Luna `reset`: the same villain, from the start.
    fn reset(&mut self) {
        *self = Self {
            message: "arcade-new-round",
            ..Self::new(self.title, self.name)
        };
    }

    fn say(&mut self, message: &'static str, value: i32, cost: i32) {
        self.message = message;
        self.value = value;
        self.cost = cost;
    }

    /// Luna `Topic`: the player's move. Nothing while the villain is yet to answer.
    pub fn play(&mut self, op: ArcadeOp, now: f32, rng: &mut GameRng) {
        if op == ArcadeOp::NewGame {
            self.reset();
            return;
        }
        if self.game_over || self.answer_at.is_some() {
            return;
        }
        match op {
            ArcadeOp::Attack => {
                let amount = rng.range(2, 6);
                self.say("arcade-attack", amount, 0);
                self.enemy_hp -= amount;
            }
            ArcadeOp::Heal => {
                let cost = rng.range(1, 3);
                let amount = rng.range(6, 8);
                self.say("arcade-heal", amount, cost);
                self.player_mp -= cost;
                self.player_hp += amount;
            }
            ArcadeOp::Recharge => {
                let amount = rng.range(4, 7);
                self.say("arcade-recharge", amount, 0);
                self.player_mp += amount;
            }
            ArcadeOp::NewGame => {}
        }
        self.answer_at = Some(now + ANSWER_DS);
    }

    /// Luna `arcade_action`: the villain's answer. Whether the player has just won.
    pub fn answer(&mut self, rng: &mut GameRng) -> bool {
        self.answer_at = None;
        if self.enemy_mp <= 0 || self.enemy_hp <= 0 {
            self.game_over = true;
            self.say("arcade-won", 0, 0);
            return true;
        }
        if self.enemy_mp <= 5 && rng.prob(70.0) {
            let amount = rng.range(2, 3);
            self.say("arcade-steal", amount, 0);
            self.player_mp -= amount;
            if self.player_mp <= 0 {
                self.game_over = true;
                self.say("arcade-drained", 0, 0);
                return false;
            }
        } else if self.enemy_hp <= 10 && self.enemy_mp > 4 {
            self.say("arcade-enemy-heals", 4, 0);
            self.enemy_hp += 4;
            self.enemy_mp -= 4;
        } else {
            let amount = rng.range(3, 6);
            self.say("arcade-enemy-attacks", amount, 0);
            self.player_hp -= amount;
        }
        if self.player_mp < 0 || self.player_hp <= 0 {
            self.game_over = true;
            self.say("arcade-crushed", 0, 0);
        }
        false
    }
}

pub fn spawn_arcade(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    let mut rng = world.resource_mut::<GameRng>();
    let title = TITLES[(rng.range(1, TITLES.len() as i32) - 1) as usize];
    let name = NAMES[(rng.range(1, NAMES.len() as i32) - 1) as usize];
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(ARCADE, "arcade"),
            Dense,
            Arcade::new(title, name),
            PowerUser::new(Channel::Equipment, 50.0),
        ))
        .id()
}

/// Moves the players asked for, the villains' answers, and prizes.
pub fn run_arcades(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut players = world.query::<(&GridPos, &mut PendingArcadeOps)>();
    let moves: Vec<(TilePos, Entity, ArcadeOp)> = players
        .iter_mut(world)
        .flat_map(|(pos, mut ops)| {
            let at = pos.0;
            std::mem::take(&mut ops.0)
                .into_iter()
                .map(move |(machine, op)| (at, machine, op))
        })
        .collect();
    for (at, machine, op) in moves {
        let close = world.get::<GridPos>(machine).is_some_and(|p| in_reach(at, p.0));
        let powered = world.get::<PowerUser>(machine).is_some_and(|p| p.powered);
        if !close || !powered {
            continue;
        }
        world.resource_scope(|world, mut rng: Mut<GameRng>| {
            if let Some(mut game) = world.get_mut::<Arcade>(machine) {
                game.play(op, now, &mut rng);
            }
        });
    }
    let mut games = world.query::<(Entity, &Arcade, &GridPos)>();
    let due: Vec<(Entity, TilePos)> = games
        .iter(world)
        .filter(|(_, g, _)| g.answer_at.is_some_and(|t| now >= t))
        .map(|(e, _, p)| (e, p.0))
        .collect();
    for (machine, at) in due {
        let won = world.resource_scope(|world, mut rng: Mut<GameRng>| {
            world
                .get_mut::<Arcade>(machine)
                .is_some_and(|mut game| game.answer(&mut rng))
        });
        if won {
            let index = (world.resource_mut::<GameRng>().range(1, PRIZES.len() as i32) - 1) as usize;
            let spec = rs13_world::ObjectSpec::new(PRIZES[index]);
            let _ = crate::objects::spawn_object(world, &spec, at);
        }
    }
}

/// Show each player the game they have open.
pub fn update_arcade_views(
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut ArcadePanelOut,
        Option<&Vitals>,
    )>,
    games: Query<(&GridPos, &Arcade, &NetId, &PowerUser)>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| games.get(v.0).ok());
        let view = target.and_then(|(at, game, id, power)| {
            let usable =
                power.powered && in_reach(pos.0, at.0) && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then(|| ArcadePanelView {
                target: *id,
                title: game.title.to_owned(),
                villain: game.name.to_owned(),
                message: game.message.to_owned(),
                value: game.value,
                cost: game.cost,
                player_hp: game.player_hp,
                player_mp: game.player_mp,
                enemy_hp: game.enemy_hp,
                game_over: game.game_over,
            })
        });
        if view.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != view {
            out.0 = view;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> (Arcade, GameRng) {
        (Arcade::new("lord", "jeff"), GameRng::seeded(11))
    }

    #[test]
    fn nothing_can_be_done_until_the_villain_answers() {
        let (mut game, mut rng) = game();
        game.play(ArcadeOp::Attack, 0.0, &mut rng);
        let hp = game.enemy_hp;
        assert!(hp < 45);
        game.play(ArcadeOp::Attack, 1.0, &mut rng);
        assert_eq!(game.enemy_hp, hp);
        assert_eq!(game.answer_at, Some(ANSWER_DS));
    }

    #[test]
    fn beating_the_villain_wins_and_a_new_game_starts_over() {
        let (mut game, mut rng) = game();
        game.enemy_hp = 0;
        assert!(game.answer(&mut rng));
        assert!(game.game_over);
        game.play(ArcadeOp::Attack, 0.0, &mut rng);
        assert_eq!(game.answer_at, None, "it's over");
        game.play(ArcadeOp::NewGame, 0.0, &mut rng);
        assert!(!game.game_over);
        assert_eq!((game.player_hp, game.enemy_hp, game.enemy_mp), (30, 45, 20));
    }

    #[test]
    fn losing_all_health_ends_the_game_without_a_prize() {
        let (mut game, mut rng) = game();
        game.player_hp = 1;
        assert!(!game.answer(&mut rng));
        assert!(game.game_over);
        assert_eq!(game.message, "arcade-crushed");
    }

    #[test]
    fn a_low_villain_heals_itself_when_it_has_the_power() {
        let (mut game, mut rng) = game();
        game.enemy_hp = 8;
        game.enemy_mp = 10;
        game.answer(&mut rng);
        assert_eq!((game.enemy_hp, game.enemy_mp), (12, 6));
    }

    #[test]
    fn a_drained_villain_steals_power_until_the_player_runs_out() {
        let (mut game, mut rng) = game();
        game.enemy_mp = 5;
        game.player_mp = 1;
        let mut guard = 0;
        while !game.game_over && guard < 100 {
            game.answer(&mut rng);
            guard += 1;
        }
        assert!(game.game_over);
    }
}
