//! Universal player client.
//!
//! Usage: `rs13-client [SERVER_ADDR]` (default `127.0.0.1:13013`).
//! Arrow keys or WASD move; hold Shift to walk instead of run.
//!
//! Placeholder graphics: tiles and players are flat coloured squares until the
//! Luna sprite import lands.

use std::net::SocketAddr;

use bevy::prelude::*;
use rs13_network::client::{ClientNetworkPlugin, connect};
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{IntentChannel, MoveInput, Player};
use rs13_world::{Dir, GridPos, MapLayer, TilePos};

/// On-screen size of one tile, in pixels.
const TILE_PX: f32 = 32.0;
/// Visual glide speed between tiles. A little faster than running so the sprite
/// never lags a full tile behind the server.
const GLIDE_TILES_PER_SEC: f32 = 5.0;

#[derive(Resource)]
struct ServerAddr(SocketAddr);

/// The player entity this client controls.
#[derive(Component)]
struct OwnPlayer;

fn main() {
    let server = std::env::args()
        .nth(1)
        .map(|a| a.parse().expect("SERVER_ADDR must look like 127.0.0.1:13013"))
        .unwrap_or(SocketAddr::from(([127, 0, 0, 1], rs13_network::DEFAULT_PORT)));

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("rust-station13 (protocol {})", rs13_network::PROTOCOL_VERSION),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ClientNetworkPlugin)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(ServerAddr(server))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                draw_map,
                add_player_sprites,
                mark_own_player,
                send_move_input,
                glide,
                follow_own_player,
            )
                .chain(),
        )
        .run();
}

fn setup(mut commands: Commands, server: Res<ServerAddr>) -> Result {
    commands.spawn(Camera2d);
    connect(&mut commands, server.0, unique_client_id())?;
    info!(server = %server.0, "connecting");
    Ok(())
}

/// Distinct per process without pulling in a random number generator.
fn unique_client_id() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    nanos ^ (u64::from(std::process::id()) << 32)
}

fn world_pos(pos: TilePos, z: f32) -> Vec3 {
    Vec3::new(pos.x as f32 * TILE_PX, pos.y as f32 * TILE_PX, z)
}

fn tile_color(id: &str) -> Color {
    match id {
        "wall" => Color::srgb(0.50, 0.52, 0.55),
        "floor" => Color::srgb(0.24, 0.25, 0.27),
        "space" => Color::srgb(0.02, 0.02, 0.06),
        _ => Color::srgb(1.0, 0.0, 1.0),
    }
}

/// Spawn one square per tile when the map arrives.
fn draw_map(mut commands: Commands, maps: Query<&MapLayer, Added<MapLayer>>) {
    for MapLayer(map) in &maps {
        info!(width = map.width, height = map.height, "map received");
        for y in 0..map.height as i32 {
            for x in 0..map.width as i32 {
                let pos = TilePos::new(x, y, map.z);
                let Some(tile) = map.tile(pos) else { continue };
                commands.spawn((
                    Sprite::from_color(tile_color(&tile.id), Vec2::splat(TILE_PX - 1.0)),
                    Transform::from_translation(world_pos(pos, 0.0)),
                ));
            }
        }
    }
}

fn add_player_sprites(mut commands: Commands, players: Query<(Entity, &GridPos), Added<Player>>) {
    for (entity, pos) in &players {
        commands.entity(entity).insert((
            Sprite::from_color(Color::srgb(0.35, 0.55, 0.95), Vec2::splat(TILE_PX * 0.7)),
            Transform::from_translation(world_pos(pos.0, 1.0)),
        ));
    }
}

fn mark_own_player(mut commands: Commands, mut own: Query<(Entity, &mut Sprite), Added<Controlled>>) {
    for (entity, mut sprite) in &mut own {
        sprite.color = Color::srgb(0.35, 0.85, 0.40);
        info!("own player received");
        commands.entity(entity).insert(OwnPlayer);
    }
}

/// Send the held direction to the server whenever it changes.
fn send_move_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut sender: Query<&mut MessageSender<MoveInput>, (With<Client>, With<Connected>)>,
    mut last_sent: Local<Option<MoveInput>>,
) {
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    let held = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
    let dx = held(KeyCode::KeyD, KeyCode::ArrowRight) as i32 - held(KeyCode::KeyA, KeyCode::ArrowLeft) as i32;
    let dy = held(KeyCode::KeyW, KeyCode::ArrowUp) as i32 - held(KeyCode::KeyS, KeyCode::ArrowDown) as i32;
    let input = MoveInput {
        dir: Dir::from_offset(dx, dy),
        run: !keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
    };
    if *last_sent != Some(input) {
        sender.send::<IntentChannel>(input);
        *last_sent = Some(input);
    }
}

/// Slide sprites towards their server position, like BYOND's glide.
fn glide(time: Res<Time>, mut movers: Query<(&GridPos, &mut Transform), With<Player>>) {
    let max_step = GLIDE_TILES_PER_SEC * TILE_PX * time.delta_secs();
    for (pos, mut transform) in &mut movers {
        let target = world_pos(pos.0, transform.translation.z);
        let offset = target - transform.translation;
        // Teleports and large corrections snap instead of sliding across the map.
        if offset.length() > TILE_PX * 2.0 {
            transform.translation = target;
        } else {
            transform.translation += offset.clamp_length_max(max_step);
        }
    }
}

fn follow_own_player(
    own: Query<&Transform, (With<OwnPlayer>, Without<Camera2d>)>,
    mut camera: Single<&mut Transform, With<Camera2d>>,
) {
    if let Ok(player) = own.single() {
        camera.translation.x = player.translation.x;
        camera.translation.y = player.translation.y;
    }
}
