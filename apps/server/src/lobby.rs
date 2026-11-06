//! The lobby and the start of a round (Luna's `new_player`, `gameticker/setup`). Someone connecting to
//! a server running rounds waits here, with no mob. Spec: `docs/reference/round.md`.

use bevy::prelude::*;
use rs13_classic::actions::Outbox;
use rs13_classic::escape::{EscapePods, PodState};
use rs13_classic::jobs::{free_post, next_start, open_posts, take_up};
use rs13_classic::names::sanitize_name;
use rs13_classic::objects::spawn_player_mob;
use rs13_classic::rng::GameRng;
use rs13_classic::round::{Phase, RESTART_DS, Round, pick_mode};
use rs13_classic::time::WorldTime;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, LobbyPanel, Notice, NoticeChannel, Player};
use rs13_world::{Appearance, CharacterPrefs, DisplayName, LobbyView, MapLayer, TilePos};

use crate::{Controls, NameChosen, SentPanel, SentStatus};

/// On a client's link while it waits in the lobby.
#[derive(Component, Debug, Default)]
pub struct InLobby {
    pub ready: bool,
    /// The job they'd like: a line of `jobs()`.
    pub prefer: Option<usize>,
    pub name: Option<String>,
    /// What they were last sent.
    sent: Option<LobbyView>,
    /// Asked to start, with the round on.
    joining: bool,
    /// Their character setup.
    pub prefs: CharacterPrefs,
    /// Would rather watch.
    observing: bool,
}

/// A player's character setup, kept once they've left the lobby: they look the same when they respawn.
#[derive(Component, Debug, Clone, Copy)]
pub struct SavedPrefs(pub CharacterPrefs);

/// Read what people in the lobby ask for.
pub fn read_lobby_inputs(
    mut links: Query<(Entity, &mut InLobby, &mut MessageReceiver<ActionInput>), Without<Controls>>,
    mut votes: ResMut<crate::vote::VoteInbox>,
    mut ooc: ResMut<OocInbox>,
    mut admin: ResMut<crate::admin::AdminInbox>,
) {
    use crate::admin::AdminInput;
    for (link, mut lobby, mut receiver) in &mut links {
        for input in receiver.receive() {
            match input {
                ActionInput::Ready(ready) => lobby.ready = ready,
                ActionInput::SetPrefs(prefs) => lobby.prefs = prefs,
                ActionInput::Observe => lobby.observing = true,
                ActionInput::Vote(action) => votes.0.push((link, action)),
                ActionInput::Ooc(text) => ooc.0.push((link, text)),
                ActionInput::Hello(secret) => admin.0.push((link, AdminInput::Hello(secret))),
                ActionInput::AdminHelp(text) => admin.0.push((link, AdminInput::Help(text))),
                ActionInput::Admin(line) => admin.0.push((link, AdminInput::Command(line))),
                ActionInput::JoinRound => {
                    lobby.ready = true;
                    lobby.joining = true;
                }
                ActionInput::ChooseJob { line } => lobby.prefer = line.map(usize::from),
                ActionInput::SetName(name) | ActionInput::Say(name) => {
                    if let Some(clean) = sanitize_name(&name) {
                        lobby.name = Some(clean);
                    }
                }
                _ => {}
            }
        }
    }
}

/// What people said out of character this tick, by link.
#[derive(Resource, Debug, Default)]
pub struct OocInbox(pub Vec<(Entity, String)>);

/// Pass on what's said out of character to every client, in the lobby or not (Luna `ooc`). Those who've
/// given no name yet are "someone": there are no accounts to show instead.
pub fn send_ooc(
    mut inbox: ResMut<OocInbox>,
    speakers: Query<(
        Option<&Controls>,
        Option<&InLobby>,
        Has<rs13_classic::combat::Muted>,
        Has<crate::admin::AdminLevel>,
    )>,
    config: Res<crate::admin::AdminConfig>,
    names: Query<&DisplayName>,
    mut links: Query<&mut MessageSender<Notice>, With<RemoteId>>,
) {
    for (link, typed) in inbox.0.drain(..) {
        let Ok((controls, lobby, muted, admin)) = speakers.get(link) else {
            continue;
        };
        // Admins can always speak (Luna's `ooc`).
        if muted || (!config.ooc_on && !admin) {
            continue;
        }
        let cleaned: String = typed
            .chars()
            .filter(|c| !c.is_control())
            .take(rs13_classic::speech::MAX_MESSAGE_LEN)
            .collect();
        let text = cleaned.trim();
        if text.is_empty() {
            continue;
        }
        let name = controls
            .and_then(|c| names.get(c.0).ok())
            .map(|n| n.0.clone())
            .or_else(|| lobby.and_then(|l| l.name.clone()));
        let msg = LocMsg::new("ooc-say").arg("text", text);
        let msg = match name {
            Some(name) => msg.arg("name", name),
            None => msg.msg("name", "ooc-unnamed"),
        };
        for mut sender in &mut links {
            sender.send::<NoticeChannel>(Notice(msg.clone()));
        }
    }
}

/// Put someone from the lobby into the world, in the job they'd like if it's free. False if there's
/// no room for them.
fn enter_world(world: &mut World, link: Entity, starts: &[TilePos], mode: &'static str) -> bool {
    let Some(lobby) = world.get::<InLobby>(link) else {
        return false;
    };
    let (prefer, name, prefs) = (lobby.prefer, lobby.name.clone(), lobby.prefs);
    // The job they asked for, or else their choices in order.
    let wanted: Vec<String> = prefer
        .into_iter()
        .chain(prefs.jobs.iter().flatten().map(|line| usize::from(*line)))
        .filter_map(|line| rs13_classic::jobs::jobs().get(line))
        .map(|job| job.id.clone())
        .collect();
    let at = wanted
        .iter()
        .find_map(|job| free_post(world, job))
        .or_else(|| next_start(world, starts));
    let (Some(at), Some(remote)) = (at, world.get::<RemoteId>(link).map(|r| r.0)) else {
        return false;
    };
    let mob = spawn_player_mob(world, at);
    take_up(world, mob, at);
    apply_prefs(world, mob, &prefs);
    world.entity_mut(link).insert(SavedPrefs(prefs));
    world.entity_mut(mob).insert((
        Player {
            client_id: remote.to_bits(),
        },
        ControlledBy {
            owner: link,
            lifetime: Lifetime::SessionBased,
        },
    ));
    if let Some(name) = name {
        world.entity_mut(mob).insert((DisplayName(name), NameChosen));
    }
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.insert(0, LocMsg::new("round-mode").arg("mode", mode));
        // Each mode's word of welcome (Luna's `announce`).
        match mode {
            "extended" => outbox.0.insert(1, LocMsg::new("round-mode-extended")),
            "sandbox" => outbox.0.insert(1, LocMsg::new("round-mode-sandbox")),
            "meteor" => outbox.0.insert(1, rs13_classic::meteor::warning()),
            "autotraitor" => outbox.0.insert(1, rs13_classic::autotraitor::announcement()),
            "restructuring" => {
                for (i, line) in rs13_classic::restructuring::announcement()
                    .into_iter()
                    .enumerate()
                {
                    outbox.0.insert(1 + i, line);
                }
            }
            _ => {}
        }
    }
    world.entity_mut(link).remove::<InLobby>().insert((
        Controls(mob),
        SentStatus::default(),
        SentPanel::default(),
    ));
    if let Some(mut sender) = world.get_mut::<MessageSender<LobbyPanel>>(link) {
        sender.send::<NoticeChannel>(LobbyPanel(None));
    }
    info!(client = remote.to_bits(), x = at.x, y = at.y, "player joined");
    true
}

/// The countdown, the start of the round, and people joining once it's on (Luna `pregame`, `setup`).
pub fn run_round(world: &mut World) {
    let Some(round) = world.get_resource::<Round>().copied() else {
        return;
    };
    let now = world.resource::<WorldTime>().deciseconds();
    let mut maps = world.query::<&MapLayer>();
    let starts: Vec<TilePos> = maps
        .iter(world)
        .flat_map(|m| m.0.spawns.iter().copied())
        .collect();
    let mut links = world.query::<(Entity, &InLobby)>();
    let watching: Vec<Entity> = links
        .iter(world)
        .filter(|(_, l)| l.observing)
        .map(|(e, _)| e)
        .collect();
    for link in watching {
        enter_as_observer(world, link, &starts);
    }
    let mode = match round.phase {
        Phase::Pregame { starts_at } => {
            if now < starts_at {
                return;
            }
            // Those with a job in mind choose first.
            let mut ready: Vec<(bool, Entity)> = links
                .iter(world)
                .filter(|(_, l)| l.ready)
                .map(|(e, l)| (l.prefer.is_none(), e))
                .collect();
            ready.sort();
            let mode = round
                .forced_mode
                .unwrap_or_else(|| pick_mode(&mut world.resource_mut::<GameRng>()));
            // A traitor round needs someone there to start with: late joiners never are one.
            // A sandbox needs no one: there's nothing to set up for those who are there.
            let mode = if ready.is_empty() && mode != "sandbox" {
                "extended"
            } else {
                mode
            };
            world.resource_mut::<Round>().phase = Phase::Playing { mode };
            info!(mode, "round started");
            if mode == "derelict" {
                // Wrecked before anyone's aboard (Luna's `pre_setup`).
                rs13_classic::derelict::wreck(world);
            }
            for (_, link) in ready {
                enter_world(world, link, &starts, mode);
            }
            if mode == "traitor" {
                rs13_classic::traitor::choose_traitors(world);
            }
            if mode == "nuclear" {
                rs13_classic::nuke::setup_round(world);
            }
            if mode == "blob" {
                rs13_classic::blob::setup_round(world);
            }
            if mode == "changeling" {
                rs13_classic::changeling::setup_round(world);
            }
            if mode == "autotraitor" {
                rs13_classic::autotraitor::setup_round(world);
            }
            if mode == "derelict" {
                rs13_classic::derelict::setup_round(world);
            }
            if mode == "monkey" {
                rs13_classic::monkey_round::setup_round(world);
            }
            if mode == "restructuring" {
                rs13_classic::restructuring::setup_round(world);
            }
            if mode == "revolution" || mode == "rp-revolution" {
                let rp = mode == "rp-revolution";
                if !rs13_classic::revolution::setup_round(world, rp) {
                    world.resource_mut::<Round>().phase = Phase::Playing { mode: "extended" };
                }
            }
            return;
        }
        Phase::Finished { restart_at } => {
            if now >= restart_at {
                world.write_message(AppExit::from_code(crate::RESTART_CODE));
            }
            return;
        }
        Phase::Playing { mode } => {
            // A sandbox round never finishes by itself (Luna's `check_finished`). A nuclear blast ends any.
            let pods_home = mode != "sandbox" && world.resource::<EscapePods>().state == PodState::Arrived;
            let decided = rs13_classic::revolution::over(world)
                || rs13_classic::changeling::over(world)
                || rs13_classic::blob::over(world);
            if pods_home || decided || world.resource::<rs13_classic::nuke::NukeState>().detonated {
                world.resource_mut::<Round>().phase = Phase::Finished {
                    restart_at: now + RESTART_DS,
                };
                info!("round over");
                return;
            }
            mode
        }
    };
    let joining: Vec<Entity> = links
        .iter(world)
        .filter(|(_, l)| l.joining)
        .map(|(e, _)| e)
        .collect();
    for link in joining {
        enter_world(world, link, &starts, mode);
        if let Some(mob) = world.get::<Controls>(link).map(|c| c.0) {
            rs13_classic::autotraitor::late_join(world, mob);
            rs13_classic::derelict::late_join(world, mob);
        }
        if let Some(mut lobby) = world.get_mut::<InLobby>(link) {
            lobby.joining = false;
        }
    }
}

/// Make someone look as they've set up, and note whether they'd like to be a traitor.
pub fn apply_prefs(world: &mut World, mob: Entity, prefs: &CharacterPrefs) {
    if let Some(mut appearance) = world.get_mut::<Appearance>(mob) {
        appearance.sprite = if prefs.look.female { "human-f" } else { "human" }.to_owned();
    }
    world.entity_mut(mob).insert(prefs.look);
    if prefs.be_nuke {
        world.entity_mut(mob).insert(rs13_classic::nuke::WantsNuke);
    }
    if prefs.be_traitor {
        world.entity_mut(mob).insert(rs13_classic::traitor::WantsTraitor);
    }
}

/// Into the world as a ghost, to watch (Luna's `observe`).
fn enter_as_observer(world: &mut World, link: Entity, starts: &[TilePos]) {
    let Some((name, prefs)) = world
        .get::<InLobby>(link)
        .map(|lobby| (lobby.name.clone(), lobby.prefs))
    else {
        return;
    };
    let (Some(at), Some(remote)) = (starts.first().copied(), world.get::<RemoteId>(link).map(|r| r.0)) else {
        return;
    };
    let ghost = rs13_classic::ghosts::spawn_observer(world, at, name);
    world.entity_mut(ghost).insert((
        Player {
            client_id: remote.to_bits(),
        },
        ControlledBy {
            owner: link,
            lifetime: Lifetime::SessionBased,
        },
    ));
    world.entity_mut(link).remove::<InLobby>().insert((
        Controls(ghost),
        SentStatus::default(),
        SentPanel::default(),
        SavedPrefs(prefs),
    ));
    if let Some(mut sender) = world.get_mut::<MessageSender<LobbyPanel>>(link) {
        sender.send::<NoticeChannel>(LobbyPanel(None));
    }
}

/// Show each person in the lobby how it stands.
pub fn send_lobby(world: &mut World) {
    let Some(round) = world.get_resource::<Round>().copied() else {
        return;
    };
    let now = world.resource::<WorldTime>().deciseconds();
    let starts_in = match round.phase {
        Phase::Pregame { starts_at } => Some(((starts_at - now) / 10.0).ceil().max(0.0) as u32),
        Phase::Playing { .. } | Phase::Finished { .. } => None,
    };
    let jobs = open_posts(world);
    let crew: Vec<(String, String)> = {
        let mut aboard = world.query::<(Entity, &DisplayName, &rs13_classic::jobs::HasJob)>();
        let mut all: Vec<_> = aboard
            .iter(world)
            .map(|(e, n, j)| (e, n.0.clone(), j.0.clone()))
            .collect();
        all.sort_by_key(|(e, ..)| *e);
        all.into_iter().map(|(_, n, j)| (n, j)).collect()
    };
    let mut links = world.query::<(Entity, &InLobby)>();
    let everyone: Vec<(Entity, bool, Option<usize>, Option<String>)> = links
        .iter(world)
        .map(|(e, l)| (e, l.ready, l.prefer, l.name.clone()))
        .collect();
    let players = everyone.len() as u16;
    let ready_count = everyone.iter().filter(|(_, ready, ..)| *ready).count() as u16;
    for (link, ready, prefer, name) in everyone {
        let view = LobbyView {
            starts_in,
            ready,
            ready_count,
            players,
            name,
            preference: prefer.and_then(|line| jobs.get(line)).map(|(id, _)| id.clone()),
            jobs: jobs.clone(),
            crew: crew.clone(),
        };
        let changed = world
            .get::<InLobby>(link)
            .is_some_and(|l| l.sent.as_ref() != Some(&view));
        if !changed {
            continue;
        }
        if let Some(mut sender) = world.get_mut::<MessageSender<LobbyPanel>>(link) {
            sender.send::<NoticeChannel>(LobbyPanel(Some(view.clone())));
        }
        if let Some(mut lobby) = world.get_mut::<InLobby>(link) {
            lobby.sent = Some(view);
        }
    }
}
