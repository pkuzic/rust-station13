//! Universal player client.
//!
//! Usage: `rs13-client [SERVER_ADDR] [--lang en|ru] [--name "First Last"]`
//! (default `127.0.0.1:13013`, English, a random name).
//!
//! Controls: arrow keys or WASD move (hold Shift to walk); left click uses the active
//! hand on something in reach; Shift+click examines; Q drops; X swaps hands; T opens
//! the say line (Enter sends, Esc cancels).
//!
//! Placeholder graphics: tiles and objects are flat coloured squares until the
//! Luna sprite import lands.

use std::collections::VecDeque;
use std::net::SocketAddr;

use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::text::{FontSize, FontSource};
use bevy::window::PrimaryWindow;
use rs13_classic::locales;
use rs13_i18n::{Case, Lexicon, Localizer, langid};
use rs13_network::client::{ClientNetworkPlugin, connect};
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, MoveInput, Notice, Player};
use rs13_world::{Appearance, Dir, GridPos, Hand, HandsView, MapLayer, NetId, TilePos};

/// On-screen size of one tile, in pixels.
const TILE_PX: f32 = 32.0;
/// Visual glide speed between tiles. A little faster than running so the sprite
/// never lags a full tile behind the server.
const GLIDE_TILES_PER_SEC: f32 = 5.0;
/// Lines kept in the message log.
const LOG_LINES: usize = 8;
/// A Windows 2000-era UI font with Cyrillic, with fallbacks for other systems.
const UI_FONT: &str = "Tahoma, 'DejaVu Sans', sans-serif";

/// Longest line the say box accepts; the server enforces the same limit.
const MAX_SAY_LEN: usize = 1024;

struct Args {
    server: SocketAddr,
    russian: bool,
    name: Option<String>,
}

/// The name to ask for once connected.
#[derive(Resource)]
struct WantedName(Option<String>);

/// The say line: open while the player is typing.
#[derive(Resource, Default)]
struct ChatInput {
    open: bool,
    buffer: String,
}

#[derive(Component)]
struct ChatLine;

#[derive(Resource)]
struct ServerAddr(SocketAddr);

#[derive(Resource)]
struct Loc(Localizer);

#[derive(Resource, Default)]
struct MessageLog(VecDeque<String>);

/// The player entity this client controls.
#[derive(Component)]
struct OwnPlayer;

#[derive(Component)]
struct LogText;

#[derive(Component)]
struct HandSlot(Hand);

#[derive(Component)]
struct HandLabel(Hand);

fn main() {
    let args = parse_args();
    let (lang, ftl, nouns) = if args.russian {
        (langid!("ru"), locales::RU_MESSAGES, locales::RU_NOUNS)
    } else {
        (langid!("en"), locales::EN_MESSAGES, locales::EN_NOUNS)
    };
    let localizer = Localizer::new(
        lang,
        &[ftl],
        Lexicon::from_ron(nouns).expect("built-in nouns are valid"),
    )
    .expect("built-in messages are valid");

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
        .insert_resource(ServerAddr(args.server))
        .insert_resource(WantedName(args.name))
        .init_resource::<ChatInput>()
        .insert_resource(Loc(localizer))
        .init_resource::<MessageLog>()
        .add_systems(Startup, (setup, spawn_hud))
        .add_systems(
            Update,
            (
                draw_map,
                add_sprites,
                update_appearance,
                show_only_placed,
                mark_own_player,
                send_name,
                chat_typing,
                (send_move_input, send_hotkeys, send_clicks),
                receive_notices,
                glide,
                follow_own_player,
                update_hands_bar,
                update_log,
                update_chat_line,
            )
                .chain(),
        )
        .run();
}

fn parse_args() -> Args {
    let mut args = Args {
        server: SocketAddr::from(([127, 0, 0, 1], rs13_network::DEFAULT_PORT)),
        russian: false,
        name: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--lang" => args.russian = it.next().as_deref() == Some("ru"),
            "--name" => args.name = it.next(),
            addr => args.server = addr.parse().expect("SERVER_ADDR must look like 127.0.0.1:13013"),
        }
    }
    args
}

fn ui_text_font(size: f32) -> TextFont {
    TextFont {
        font: FontSource::Families(UI_FONT.into()),
        font_size: FontSize::Px(size),
        ..default()
    }
}

fn setup(mut commands: Commands, server: Res<ServerAddr>) -> Result {
    commands.spawn(Camera2d);
    connect(&mut commands, server.0, unique_client_id())?;
    info!(server = %server.0, "connecting");
    Ok(())
}

/// Classic grey panels: message log above, two hand slots below.
fn spawn_hud(mut commands: Commands) {
    let panel_grey = Color::srgb(0.75, 0.75, 0.72);
    let hand_slot = |hand: Hand| {
        (
            HandSlot(hand),
            Node {
                width: Val::Px(160.0),
                height: Val::Px(28.0),
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::horizontal(Val::Px(6.0)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(panel_grey),
            BorderColor::all(Color::srgb(0.4, 0.4, 0.4)),
            children![(
                HandLabel(hand),
                Text::new(""),
                ui_text_font(14.0),
                TextColor(Color::BLACK)
            )],
        )
    };
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexEnd,
            padding: UiRect::all(Val::Px(8.0)),
            row_gap: Val::Px(6.0),
            ..default()
        },
        children![
            (
                LogText,
                Text::new(""),
                ui_text_font(15.0),
                TextColor(Color::srgb(0.85, 0.85, 0.85))
            ),
            (
                ChatLine,
                Text::new(""),
                ui_text_font(15.0),
                TextColor(Color::srgb(1.0, 1.0, 0.75))
            ),
            (
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                },
                children![hand_slot(Hand::Left), hand_slot(Hand::Right)],
            ),
        ],
    ));
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

/// Placeholder look of an object: colour, size and draw layer.
fn object_look(appearance: &Appearance) -> (Color, f32, f32) {
    match (appearance.sprite.as_str(), appearance.state.as_str()) {
        ("human", _) => (Color::srgb(0.35, 0.55, 0.95), TILE_PX * 0.7, 3.0),
        ("airlock", "deny") => (Color::srgb(0.85, 0.20, 0.15), TILE_PX - 1.0, 2.0),
        ("airlock", "open") => (Color::srgb(0.22, 0.20, 0.14), TILE_PX - 1.0, 2.0),
        ("airlock", "opening" | "closing") => (Color::srgb(0.70, 0.58, 0.30), TILE_PX - 1.0, 2.0),
        ("airlock", _) => (Color::srgb(0.55, 0.45, 0.25), TILE_PX - 1.0, 2.0),
        ("crowbar", _) => (Color::srgb(0.80, 0.20, 0.15), 14.0, 1.0),
        ("wrench", _) => (Color::srgb(0.55, 0.60, 0.70), 14.0, 1.0),
        ("screwdriver", _) => (Color::srgb(0.90, 0.80, 0.20), 14.0, 1.0),
        ("wirecutters", _) => (Color::srgb(0.95, 0.55, 0.15), 14.0, 1.0),
        ("oxygen-tank", _) => (Color::srgb(0.25, 0.45, 0.95), 14.0, 1.0),
        ("metal-sheet", _) => (Color::srgb(0.80, 0.80, 0.82), 14.0, 1.0),
        ("id-card", _) => (Color::srgb(0.30, 0.80, 0.85), 12.0, 1.0),
        _ => (Color::srgb(1.0, 0.0, 1.0), 14.0, 1.0),
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

fn add_sprites(
    mut commands: Commands,
    objects: Query<(Entity, &Appearance, Option<&GridPos>), Added<Appearance>>,
) {
    for (entity, appearance, pos) in &objects {
        let (color, size, z) = object_look(appearance);
        let pos = pos.map(|p| p.0).unwrap_or(TilePos::new(0, 0, 0));
        commands.entity(entity).insert((
            Sprite::from_color(color, Vec2::splat(size)),
            Transform::from_translation(world_pos(pos, z)),
        ));
    }
}

fn update_appearance(mut objects: Query<(&Appearance, &mut Sprite, Has<OwnPlayer>), Changed<Appearance>>) {
    for (appearance, mut sprite, own) in &mut objects {
        if !own {
            sprite.color = object_look(appearance).0;
        }
    }
}

/// Things inside something else (a held item) have no `GridPos` and are not drawn.
fn show_only_placed(mut objects: Query<(&mut Visibility, Has<GridPos>), With<Appearance>>) {
    for (mut visibility, placed) in &mut objects {
        visibility.set_if_neq(if placed {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

fn mark_own_player(
    mut commands: Commands,
    mut own: Query<(Entity, &mut Sprite), (Added<Controlled>, With<Player>)>,
) {
    for (entity, mut sprite) in &mut own {
        sprite.color = Color::srgb(0.35, 0.85, 0.40);
        commands.entity(entity).insert(OwnPlayer);
        info!("own player received");
    }
}

/// Ask for the chosen name once, right after connecting.
fn send_name(
    mut wanted: ResMut<WantedName>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    if let Some(name) = wanted.0.take() {
        sender.send::<IntentChannel>(ActionInput::SetName(name));
    }
}

/// T opens the say line; typed text goes into it; Enter sends, Esc cancels.
fn chat_typing(
    mut keys: MessageReader<KeyboardInput>,
    mut chat: ResMut<ChatInput>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    for key in keys.read() {
        if key.state != ButtonState::Pressed {
            continue;
        }
        if !chat.open {
            if key.key_code == KeyCode::KeyT && !key.repeat {
                chat.open = true;
                chat.buffer.clear();
            }
            continue;
        }
        match key.key_code {
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let line = std::mem::take(&mut chat.buffer);
                chat.open = false;
                if !line.trim().is_empty()
                    && let Ok(mut sender) = sender.single_mut()
                {
                    sender.send::<IntentChannel>(ActionInput::Say(line));
                }
            }
            KeyCode::Escape => {
                chat.open = false;
                chat.buffer.clear();
            }
            KeyCode::Backspace => {
                chat.buffer.pop();
            }
            _ => {
                if let Some(text) = &key.text {
                    let room = MAX_SAY_LEN.saturating_sub(chat.buffer.chars().count());
                    let typed: String = text.chars().filter(|c| !c.is_control()).take(room).collect();
                    chat.buffer.push_str(&typed);
                }
            }
        }
    }
}

/// Send the held direction to the server whenever it changes.
fn send_move_input(
    chat: Res<ChatInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut sender: Query<&mut MessageSender<MoveInput>, (With<Client>, With<Connected>)>,
    mut last_sent: Local<Option<MoveInput>>,
) {
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    // While typing, letters are text, not movement.
    let held = |a: KeyCode, b: KeyCode| !chat.open && (keys.pressed(a) || keys.pressed(b));
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

fn send_hotkeys(
    chat: Res<ChatInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    // `chat` may have just closed this frame on Enter; only a closed line takes hotkeys.
    if chat.open || chat.is_changed() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyQ) {
        sender.send::<IntentChannel>(ActionInput::Drop);
    }
    if keys.just_pressed(KeyCode::KeyX) {
        sender.send::<IntentChannel>(ActionInput::SwapHands);
    }
}

/// Left click: use the active hand on the topmost thing on the clicked tile.
/// Shift+click: examine it. The server checks reach.
fn send_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    objects: Query<(&NetId, &GridPos, &Appearance)>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let (camera, camera_transform) = *camera;
    let Ok(world) = camera.viewport_to_world_2d(camera_transform, cursor) else {
        return;
    };
    let tile = (
        (world.x / TILE_PX).round() as i32,
        (world.y / TILE_PX).round() as i32,
    );

    // Lowest draw layer first: items over doors over mobs, so the item on a tile wins.
    let target = objects
        .iter()
        .filter(|(_, pos, _)| (pos.0.x, pos.0.y) == tile)
        .min_by(|a, b| object_look(a.2).2.total_cmp(&object_look(b.2).2))
        .map(|(id, ..)| *id);
    let Some(target) = target else { return };
    let examine = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    sender.send::<IntentChannel>(if examine {
        ActionInput::Examine(target)
    } else {
        ActionInput::UseOn(target)
    });
}

fn receive_notices(
    loc: Res<Loc>,
    mut log: ResMut<MessageLog>,
    mut receivers: Query<&mut MessageReceiver<Notice>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        for Notice(msg) in receiver.receive() {
            log.0.push_back(loc.0.format_msg(&msg));
            while log.0.len() > LOG_LINES {
                log.0.pop_front();
            }
        }
    }
}

/// Slide mobs towards their server position, like BYOND's glide; other objects snap.
fn glide(time: Res<Time>, mut movers: Query<(&GridPos, &mut Transform, Has<Player>)>) {
    let max_step = GLIDE_TILES_PER_SEC * TILE_PX * time.delta_secs();
    for (pos, mut transform, is_player) in &mut movers {
        let target = world_pos(pos.0, transform.translation.z);
        let offset = target - transform.translation;
        // Teleports and large corrections snap instead of sliding across the map.
        if !is_player || offset.length() > TILE_PX * 2.0 {
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

fn update_hands_bar(
    loc: Res<Loc>,
    own: Query<&HandsView, With<OwnPlayer>>,
    items: Query<(&NetId, &Appearance)>,
    mut slots: Query<(&HandSlot, &mut BorderColor)>,
    mut labels: Query<(&HandLabel, &mut Text)>,
) {
    let Ok(view) = own.single() else { return };
    let name = |id: Option<NetId>| {
        id.and_then(|id| items.iter().find(|(n, _)| **n == id))
            .map(|(_, a)| loc.0.noun(&a.sprite, Case::Nom))
            .unwrap_or_else(|| "—".to_owned())
    };
    for (HandLabel(hand), mut text) in &mut labels {
        let (label_id, held) = match hand {
            Hand::Left => ("hud-hand-left", view.left),
            Hand::Right => ("hud-hand-right", view.right),
        };
        let label = loc.0.format(label_id, None).unwrap_or_default();
        let new = format!("{label}: {}", name(held));
        if text.0 != new {
            text.0 = new;
        }
    }
    for (HandSlot(hand), mut border) in &mut slots {
        let color = if *hand == view.active {
            Color::srgb(0.95, 0.95, 0.95)
        } else {
            Color::srgb(0.4, 0.4, 0.4)
        };
        border.set_if_neq(BorderColor::all(color));
    }
}

fn update_log(log: Res<MessageLog>, mut text: Single<&mut Text, With<LogText>>) {
    if log.is_changed() {
        text.0 = log.0.iter().cloned().collect::<Vec<_>>().join("\n");
    }
}

fn update_chat_line(loc: Res<Loc>, chat: Res<ChatInput>, mut text: Single<&mut Text, With<ChatLine>>) {
    if !chat.is_changed() {
        return;
    }
    text.0 = if chat.open {
        let prompt = loc.0.format("hud-say", None).unwrap_or_default();
        format!("{prompt} {}_", chat.buffer)
    } else {
        String::new()
    };
}
