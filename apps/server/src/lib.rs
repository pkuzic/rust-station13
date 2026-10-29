//! The dedicated server as a library, so tests can run it in-process.

use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::combat::CombatIntent;
use rs13_classic::health::Vitals;
use rs13_classic::movement::{Mob, MoveIntent};
use rs13_classic::names::sanitize_name;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::speech::PendingSpeech;
use rs13_classic::{ClassicPlugin, maps};
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::server::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, MoveInput, Notice, NoticeChannel, Player, StatusUpdate};
use rs13_network::server::{ServerNetworkPlugin, start_server};
use rs13_world::{DisplayName, GridPos, MapLayer, NetId, TileMap};

#[derive(Resource)]
struct Port(u16);

/// On a client's link entity: the mob that client controls.
#[derive(Component)]
struct Controls(Entity);

/// On a client's link entity: the status last sent to it.
#[derive(Component, Default)]
struct SentStatus(Option<StatusUpdate>);

/// On a mob whose player has already chosen a name this session.
#[derive(Component)]
struct NameChosen;

/// A server listening on UDP `port`, ready to `run()`. Logging is left to the caller.
pub fn server_app(port: u16) -> App {
    let mut app = App::new();
    app.add_plugins((
        // Wake often enough to service the network; the game itself runs on fixed ticks.
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(2))),
        StatesPlugin,
    ))
    .add_plugins((ServerNetworkPlugin, ClassicPlugin))
    .insert_resource(Port(port))
    .add_systems(Startup, (load_map, listen))
    .add_observer(replicate_net_entities)
    .add_observer(spawn_player)
    // Lightyear drops messages not read in the frame they arrive, and a frame may
    // have no fixed tick, so read inputs every frame; the game applies them on ticks.
    .add_systems(
        Update,
        (apply_move_inputs, queue_actions, send_notices, send_status),
    );
    app
}

fn load_map(world: &mut World) {
    let map = TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).expect("built-in map is valid");
    info!(
        width = map.width,
        height = map.height,
        spawns = map.spawns.len(),
        "map loaded"
    );
    for proto in spawn_map(world, map) {
        warn!(proto, "map places an unknown object; skipped");
    }
}

fn listen(mut commands: Commands, port: Res<Port>) {
    start_server(&mut commands, port.0);
    info!(
        port = port.0,
        protocol = rs13_network::PROTOCOL_VERSION,
        tick_rate = rs13_core::TICK_RATE_HZ,
        reference = rs13_classic::REFERENCE_COMMIT,
        "rs13-server listening"
    );
}

/// Everything with a `NetId` is visible to every client.
fn replicate_net_entities(trigger: On<Add<NetId>>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert(Replicate::to_clients(NetworkTarget::All));
}

/// Once a client is fully connected, give it a mob on the first free spawn point.
fn spawn_player(
    trigger: On<Add<Connected>>,
    links: Query<&RemoteId, With<ClientOf>>,
    map: Single<&MapLayer>,
    occupied: Query<&GridPos, With<Mob>>,
    mut commands: Commands,
) {
    let link = trigger.entity;
    let Ok(remote) = links.get(link) else { return };
    let client_id = remote.0.to_bits();

    let taken: Vec<_> = occupied.iter().map(|p| p.0).collect();
    let Some(spawn) = map.0.spawns.iter().find(|s| !taken.contains(s)).copied() else {
        warn!(client_id, "no free spawn point; player left without a mob");
        return;
    };

    commands.queue(move |world: &mut World| {
        let mob = spawn_player_mob(world, spawn);
        world.entity_mut(mob).insert((
            Player { client_id },
            ControlledBy {
                owner: link,
                lifetime: Lifetime::SessionBased,
            },
        ));
        if let Ok(mut link) = world.get_entity_mut(link) {
            link.insert((Controls(mob), SentStatus::default()));
        }
        info!(client_id, x = spawn.x, y = spawn.y, "player joined");
    });
}

/// Copy the latest movement request from each client onto its mob.
fn apply_move_inputs(
    mut links: Query<(&Controls, &mut MessageReceiver<MoveInput>)>,
    mut intents: Query<&mut MoveIntent>,
) {
    for (controls, mut receiver) in &mut links {
        let Some(input) = receiver.receive().last() else {
            continue;
        };
        if let Ok(mut intent) = intents.get_mut(controls.0) {
            intent.dir = input.dir;
            intent.run = input.run;
        }
    }
}

/// Turn client actions into game actions for the next tick, resolving `NetId`s.
fn queue_actions(
    mut links: Query<(&Controls, &mut MessageReceiver<ActionInput>)>,
    mut mobs: Query<(
        &mut PendingActions,
        &mut PendingSpeech,
        &mut DisplayName,
        &mut Outbox,
        Has<NameChosen>,
    )>,
    ids: Query<(Entity, &NetId)>,
    mut commands: Commands,
) {
    let resolve = |id: NetId| ids.iter().find(|(_, n)| **n == id).map(|(e, _)| e);
    for (controls, mut receiver) in &mut links {
        let Ok((mut pending, mut speech, mut name, mut outbox, name_chosen)) = mobs.get_mut(controls.0)
        else {
            receiver.receive().for_each(drop);
            continue;
        };
        for input in receiver.receive() {
            let action = match input {
                ActionInput::UseOn(id) => resolve(id).map(Action::UseOn),
                ActionInput::Examine(id) => resolve(id).map(Action::Examine),
                ActionInput::Drop => Some(Action::Drop),
                ActionInput::SwapHands => Some(Action::SwapHands),
                ActionInput::SetIntent(intent) => Some(Action::SetIntent(intent)),
                ActionInput::Say(text) => {
                    speech.0.push(text);
                    None
                }
                ActionInput::SetName(requested) => {
                    if !name_chosen {
                        match sanitize_name(&requested) {
                            Some(clean) => name.0 = clean,
                            None => outbox
                                .0
                                .push(LocMsg::new("name-rejected").arg("name", name.0.clone())),
                        }
                        commands.entity(controls.0).insert(NameChosen);
                    }
                    None
                }
            };
            // Unknown ids: the entity is gone or never existed. Ignore, as a stale click.
            if let Some(action) = action {
                pending.0.push(action);
            }
        }
    }
}

/// Deliver each mob's queued messages to the client controlling it.
fn send_notices(mut links: Query<(&Controls, &mut MessageSender<Notice>)>, mut outboxes: Query<&mut Outbox>) {
    for (controls, mut sender) in &mut links {
        let Ok(mut outbox) = outboxes.get_mut(controls.0) else {
            continue;
        };
        for msg in outbox.0.drain(..) {
            sender.send::<NoticeChannel>(Notice(msg));
        }
    }
}

/// Tell each client its own health, state and intent whenever they change.
fn send_status(
    mut links: Query<(&Controls, &mut SentStatus, &mut MessageSender<StatusUpdate>)>,
    mobs: Query<(&Vitals, &CombatIntent)>,
) {
    for (controls, mut sent, mut sender) in &mut links {
        let Ok((vitals, intent)) = mobs.get(controls.0) else {
            continue;
        };
        let status = StatusUpdate {
            // Luna shows health as a whole number.
            health: vitals.health.round() as i32,
            stat: vitals.stat,
            intent: intent.0,
        };
        if sent.0 != Some(status) {
            sender.send::<NoticeChannel>(status);
            sent.0 = Some(status);
        }
    }
}
