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
use rs13_world::{DisplayName, LobbyView, MapLayer, TilePos};

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
}

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
    let (prefer, name) = (lobby.prefer, lobby.name.clone());
    let wanted = prefer
        .and_then(|line| rs13_classic::jobs::jobs().get(line))
        .map(|job| job.id.clone());
    let at = wanted
        .and_then(|job| free_post(world, &job))
        .or_else(|| next_start(world, starts));
    let (Some(at), Some(remote)) = (at, world.get::<RemoteId>(link).map(|r| r.0)) else {
        return false;
    };
    let mob = spawn_player_mob(world, at);
    take_up(world, mob, at);
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
            let mode = if ready.is_empty() { "extended" } else { mode };
            world.resource_mut::<Round>().phase = Phase::Playing { mode };
            info!(mode, "round started");
            for (_, link) in ready {
                enter_world(world, link, &starts, mode);
            }
            if mode == "traitor" {
                rs13_classic::traitor::choose_traitors(world);
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
            if world.resource::<EscapePods>().state == PodState::Arrived {
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
        if let Some(mut lobby) = world.get_mut::<InLobby>(link) {
            lobby.joining = false;
        }
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
