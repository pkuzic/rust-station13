//! Votes to restart the server or change the mode (Luna's `game/vote`). Spec:
//! `docs/reference/round.md`.

use std::sync::Mutex;

use bevy::prelude::*;
use rs13_classic::actions::Outbox;
use rs13_classic::rng::GameRng;
use rs13_classic::round::{MODES, Round};
use rs13_classic::time::WorldTime;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{Notice, NoticeChannel, VoteAction, VotePanel};
use rs13_world::VoteView;

/// How long a vote stays open, and the least time between them, ds (Luna's `config.txt`).
pub const VOTE_PERIOD_DS: f32 = 600.0;
pub const VOTE_DELAY_DS: f32 = 6000.0;
/// How long after a vote the server restarts, ds (Luna's 5 and 10 seconds).
pub const RESTART_VOTE_DS: f32 = 50.0;
pub const MODE_VOTE_DS: f32 = 100.0;
/// The choice that picks a mode at random each round, as a server starts out.
pub const SECRET: &str = "secret";
/// The choice that leaves things as they are.
const NO_CHANGE: &str = "default";

/// The mode the players last voted for, kept for after a restart (Luna `save_mode`).
pub static VOTED_MODE: Mutex<Option<&'static str>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteKind {
    Restart,
    Mode,
}

/// How voting stands.
#[derive(Resource, Debug, Clone)]
pub struct Votes {
    pub current: Option<(VoteKind, f32)>,
    /// When the next vote can start, ds.
    pub next_at: f32,
    /// When the server restarts because of one, ds.
    pub restart_at: Option<f32>,
    pub period_ds: f32,
    pub delay_ds: f32,
}

impl Default for Votes {
    fn default() -> Self {
        Self {
            current: None,
            next_at: 0.0,
            restart_at: None,
            period_ds: VOTE_PERIOD_DS,
            delay_ds: VOTE_DELAY_DS,
        }
    }
}

/// What people asked this tick, by link.
#[derive(Resource, Debug, Default)]
pub struct VoteInbox(pub Vec<(Entity, VoteAction)>);

/// On a client's link: how they're voting, and what they were last sent.
#[derive(Component, Debug, Default)]
pub struct Voter {
    choice: Option<String>,
    sent: Option<VoteView>,
}

/// What can be voted for.
fn choices(kind: VoteKind) -> Vec<String> {
    match kind {
        VoteKind::Restart => vec!["no".to_owned(), "restart".to_owned()],
        VoteKind::Mode => [NO_CHANGE, SECRET]
            .into_iter()
            // Luna's `votable = 0` and `enabled = 0`: only ever forced, or chance for meteors.
            .chain(
                MODES
                    .iter()
                    .filter(|m| m.built && !matches!(m.id, "meteor" | "blob"))
                    .map(|m| m.id),
            )
            .map(str::to_owned)
            .collect(),
    }
}

/// How many are for each choice. Someone who hasn't voted counts as no change, which only a mode
/// vote has (Luna gives everyone `default`, and a restart vote never counts that).
fn tally(kind: VoteKind, cast: &[Option<String>]) -> Vec<(String, u16)> {
    choices(kind)
        .into_iter()
        .map(|choice| {
            let count = cast
                .iter()
                .filter(|c| c.as_deref().unwrap_or(NO_CHANGE) == choice)
                .count() as u16;
            (choice, count)
        })
        .collect()
}

/// Who's ahead: restart needs more for it than against, and a mode the most, with ties left to chance
/// when `rng` is given (Luna `calcwin`).
fn winner(kind: VoteKind, counts: &[(String, u16)], rng: Option<&mut GameRng>) -> String {
    match kind {
        VoteKind::Restart => {
            let count = |id: &str| counts.iter().find(|(c, _)| c == id).map_or(0, |(_, n)| *n);
            if count("no") < count("restart") {
                "restart".to_owned()
            } else {
                "no".to_owned()
            }
        }
        VoteKind::Mode => {
            let best = counts.iter().map(|(_, n)| *n).max().unwrap_or(0);
            let top: Vec<&String> = counts
                .iter()
                .filter(|(_, n)| *n == best)
                .map(|(c, _)| c)
                .collect();
            let at = rng.map_or(0, |rng| rng.range(0, top.len() as i32 - 1) as usize);
            top.get(at).map_or_else(|| NO_CHANGE.to_owned(), |c| (*c).clone())
        }
    }
}

fn tell_everyone(world: &mut World, msg: LocMsg) {
    let mut people = world.query::<&mut Outbox>();
    for mut outbox in people.iter_mut(world) {
        outbox.0.push(msg.clone());
    }
    // Those in the lobby have no mob to tell.
    let mut lobby = world.query_filtered::<&mut MessageSender<Notice>, With<crate::lobby::InLobby>>();
    for mut sender in lobby.iter_mut(world) {
        sender.send::<NoticeChannel>(Notice(msg.clone()));
    }
}

fn cast_votes(world: &mut World) -> Vec<Option<String>> {
    let mut voters = world.query::<&Voter>();
    voters.iter(world).map(|v| v.choice.clone()).collect()
}

fn begin(world: &mut World, kind: VoteKind, now: f32) {
    let votes = world.resource::<Votes>();
    if votes.current.is_some() || votes.restart_at.is_some() || now < votes.next_at {
        return;
    }
    if kind == VoteKind::Mode && world.get_resource::<Round>().is_none() {
        return;
    }
    let period = votes.period_ds;
    world.resource_mut::<Votes>().current = Some((kind, now + period));
    let mut voters = world.query::<&mut Voter>();
    for mut voter in voters.iter_mut(world) {
        voter.choice = None;
    }
    let what = match kind {
        VoteKind::Restart => "restart",
        VoteKind::Mode => "mode",
    };
    tell_everyone(
        world,
        LocMsg::new("vote-started")
            .arg("kind", what)
            .num("value", (period / 10.0).round() as i64),
    );
    info!(what, "vote started");
}

fn finish(world: &mut World, kind: VoteKind, now: f32) {
    let cast = cast_votes(world);
    let counts = tally(kind, &cast);
    let won = winner(kind, &counts, Some(&mut world.resource_mut::<GameRng>()));
    let mut votes = world.resource_mut::<Votes>();
    votes.current = None;
    votes.next_at = now + votes.delay_ds;
    let msg = match (kind, won.as_str()) {
        (VoteKind::Restart, "no") => LocMsg::new("vote-result-no-restart"),
        (VoteKind::Restart, _) => {
            votes.restart_at = Some(now + RESTART_VOTE_DS);
            LocMsg::new("vote-result-restart").num("value", (RESTART_VOTE_DS / 10.0) as i64)
        }
        (VoteKind::Mode, NO_CHANGE) => LocMsg::new("vote-result-no-change"),
        (VoteKind::Mode, _) => LocMsg::new("vote-result-mode").msg("mode", format!("vote-choice-{won}")),
    };
    info!(won, "vote over");
    tell_everyone(world, msg);
    if kind == VoteKind::Mode && won != NO_CHANGE {
        apply_mode(world, &won, now);
    }
}

/// Before the round, the vote settles this one. After that it's for the next, and the server restarts.
fn apply_mode(world: &mut World, won: &str, now: f32) {
    let mode = MODES.iter().find(|m| m.id == won).map(|m| m.id);
    if let Ok(mut voted) = VOTED_MODE.lock() {
        *voted = if won == SECRET { Some(SECRET) } else { mode };
    }
    let Some(mut round) = world.get_resource_mut::<Round>() else {
        return;
    };
    round.forced_mode = mode;
    if !matches!(round.phase, rs13_classic::round::Phase::Pregame { .. }) {
        world.resource_mut::<Votes>().restart_at = Some(now + MODE_VOTE_DS);
        tell_everyone(
            world,
            LocMsg::new("vote-reboot").num("value", (MODE_VOTE_DS / 10.0) as i64),
        );
    }
}

fn line_of(kind: VoteKind, line: u8) -> Option<String> {
    choices(kind).into_iter().nth(usize::from(line))
}

/// Starts and ends votes, takes them, and restarts the server if that's what was decided.
pub fn run_votes(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let asked = std::mem::take(&mut world.resource_mut::<VoteInbox>().0);
    for (link, action) in asked {
        match action {
            VoteAction::BeginRestart => begin(world, VoteKind::Restart, now),
            VoteAction::BeginMode => begin(world, VoteKind::Mode, now),
            VoteAction::Cast(line) => {
                let kind = world.resource::<Votes>().current.map(|(k, _)| k);
                let choice = kind.and_then(|k| line_of(k, line));
                if let (Some(choice), Some(mut voter)) = (choice, world.get_mut::<Voter>(link)) {
                    voter.choice = Some(choice);
                }
            }
        }
    }
    let votes = world.resource::<Votes>();
    if let Some((kind, ends_at)) = votes.current
        && now >= ends_at
    {
        finish(world, kind, now);
    }
    if world.resource::<Votes>().restart_at.is_some_and(|at| now >= at) {
        world.write_message(AppExit::from_code(crate::RESTART_CODE));
    }
}

/// Show each client how voting stands, when that changes.
pub fn send_votes(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut new_links = world.query_filtered::<Entity, (With<RemoteId>, Without<Voter>)>();
    let new: Vec<Entity> = new_links.iter(world).collect();
    for link in new {
        world.entity_mut(link).insert(Voter::default());
    }
    let votes = world.resource::<Votes>().clone();
    let can_mode = world.get_resource::<Round>().is_some();
    let cast = cast_votes(world);
    let current = votes.current.map(|(kind, ends_at)| {
        let counts = tally(kind, &cast);
        let leading = winner(kind, &counts, None);
        (kind, ends_at, counts, leading)
    });
    let shared = VoteView {
        kind: current.as_ref().map(|(kind, ..)| {
            match kind {
                VoteKind::Restart => "restart",
                VoteKind::Mode => "mode",
            }
            .to_owned()
        }),
        ends_in: current.as_ref().map_or(0, |(_, ends_at, ..)| {
            ((ends_at - now) / 10.0).ceil().max(0.0) as u32
        }),
        next_in: ((votes.next_at - now) / 10.0).ceil().max(0.0) as u32,
        choices: current
            .as_ref()
            .map_or_else(Vec::new, |(_, _, counts, _)| counts.clone()),
        leading: current.as_ref().map(|(.., leading)| leading.clone()),
        mine: None,
        can_mode,
    };
    let mut links = world.query::<(&mut Voter, &mut MessageSender<VotePanel>)>();
    for (mut voter, mut sender) in links.iter_mut(world) {
        let view = VoteView {
            mine: voter.choice.clone(),
            ..shared.clone()
        };
        if voter.sent.as_ref() != Some(&view) {
            sender.send::<NoticeChannel>(VotePanel(view.clone()));
            voter.sent = Some(view);
        }
    }
}
