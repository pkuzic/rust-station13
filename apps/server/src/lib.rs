//! The dedicated server as a library, so tests can run it in-process.

use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use rs13_classic::movement::{Mob, MoveIntent};
use rs13_classic::{ClassicPlugin, maps};
use rs13_network::lightyear::prelude::server::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{MoveInput, Player};
use rs13_network::server::{ServerNetworkPlugin, start_server};
use rs13_world::{GridPos, MapLayer, TileMap};

#[derive(Resource)]
struct Port(u16);

/// On a client's link entity: the mob that client controls.
#[derive(Component)]
struct Controls(Entity);

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
    .add_observer(spawn_player)
    // Lightyear drops messages not read in the frame they arrive, and a frame may
    // have no fixed tick, so read inputs every frame; movement applies them on ticks.
    .add_systems(Update, apply_move_inputs);
    app
}

fn load_map(mut commands: Commands) {
    let map = TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).expect("built-in map is valid");
    info!(
        width = map.width,
        height = map.height,
        spawns = map.spawns.len(),
        "map loaded"
    );
    commands.spawn((MapLayer(map), Replicate::to_clients(NetworkTarget::All)));
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

    let mob = commands
        .spawn((
            Player { client_id },
            GridPos(spawn),
            Mob::default(),
            MoveIntent::default(),
            Replicate::to_clients(NetworkTarget::All),
            ControlledBy {
                owner: link,
                lifetime: Lifetime::SessionBased,
            },
        ))
        .id();
    commands.entity(link).insert(Controls(mob));
    info!(client_id, x = spawn.x, y = spawn.y, "player joined");
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
