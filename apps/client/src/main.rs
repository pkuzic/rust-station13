//! Universal player client.
//!
//! Usage: `rs13-client [SERVER_ADDR] [--lang en|ru] [--name "First Last"]`
//! (default `127.0.0.1:13013`, English, a random name).
//!
//! Controls: arrow keys or WASD move (hold Shift to walk, Ctrl to turn on the spot); left
//! click uses the active hand on something in reach (a held item hits people on any
//! intent); Ctrl+click pulls it along; Shift+click examines; Q drops; X swaps hands; 1 / 4 set help / harm intent; T opens the say line
//! (Enter sends, Esc cancels); F3 toggles the atmos pressure overlay; F4 toggles lighting.
//!
//! The screen is laid out like the classic BYOND client: the map with Luna's HUD on the
//! left (`hud.rs`), messages on the right. Click an inventory slot to put on what the
//! active hand holds, or with an empty hand to take off what is there. E or the
//! internals indicator connects the breath mask to a gas tank.

mod alarm_panel;
mod apc_panel;
mod audio;
mod context_menu;
mod depth;
mod fov;
mod freezer_panel;
mod gas;
mod glow;
mod heat;
mod hud;
mod laser_panel;
mod lighting;
mod monitor_panel;
mod scaling;
mod shadows;
mod smes_panel;
mod space;
mod sprite_shadows;
mod sprites;
mod storage_panel;
mod wire_panel;
mod world_view;

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;

use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::text::{FontSize, FontSource};
use bevy::window::PrimaryWindow;
use rs13_classic::locales;
use rs13_classic::machines::{is_canister, is_pipe_tank};
use rs13_i18n::{Lexicon, Localizer, langid};
use rs13_network::client::{ClientNetworkPlugin, connect};
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, MoveInput, Notice, Player, StatusUpdate};
use rs13_world::{
    Appearance, AtmosView, Dir, EnvAlert, GridPos, Hand, HandsView, Hidden, Intent, MapLayer, MobStat, NetId,
    TilePos,
};
use sprites::{Look, SpriteAtlas};

/// On-screen size of one tile, in pixels.
const TILE_PX: f32 = 32.0;
/// Visual glide speed between tiles. A little faster than running so the sprite
/// never lags a full tile behind the server.
const GLIDE_TILES_PER_SEC: f32 = 5.0;
/// Lines kept in the message log.
const LOG_LINES: usize = 60;
/// Colour of the player's own character.
const OWN_PLAYER_COLOR: Color = Color::srgb(0.35, 0.85, 0.40);
/// A Windows 2000-era UI font with Cyrillic, with fallbacks for other systems.
const UI_FONT: &str = "Tahoma, 'DejaVu Sans', sans-serif";

/// Longest line the say box accepts; the server enforces the same limit.
const MAX_SAY_LEN: usize = 1024;

struct Args {
    server: SocketAddr,
    russian: bool,
    name: Option<String>,
    /// Save a screenshot here a few seconds after joining, then quit (for checking).
    screenshot: Option<String>,
    /// Seconds after joining to take it (`--shot-after`, default 4).
    shot_after: f32,
    /// Window size in logical pixels, as `WIDTHxHEIGHT`.
    size: Option<(f32, f32)>,
    /// For checking windows in screenshots: what to do once joined (`--use`, repeatable).
    use_at: Vec<UseStep>,
}

/// One `--use` step, half a second apart.
#[derive(Debug, Clone, Copy, PartialEq)]
enum UseStep {
    /// `DX,DY`: use whatever is this many tiles east and north of our player.
    Tile(i32, i32),
    /// `pull:DX,DY`: Ctrl-click it instead, to pull it.
    Pull(i32, i32),
    /// `swap`: swap hands.
    Swap,
    /// `held`: click the item in the other hand (an empty hand on a held container opens it).
    Held,
    /// `self`: click the item in the active hand, as `Z` does (Luna `attack_self`).
    InHand,
    /// `floor:DX,DY`: click the floor of that tile, whatever is on it.
    Floor(i32, i32),
    /// `wait:SECONDS`: pause before the next step.
    Wait(f32),
    /// `menu:DX,DY`: open the right-click menu on that tile.
    Menu(i32, i32),
    /// `pick:N`: pick the open menu's `N`th choice (from 1).
    Pick(usize),
}

impl UseStep {
    fn parse(s: &str) -> Option<Self> {
        let offset = |s: &str| -> Option<(i32, i32)> {
            let (x, y) = s.split_once(',')?;
            Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
        };
        match s {
            "swap" => Some(Self::Swap),
            "held" => Some(Self::Held),
            "self" => Some(Self::InHand),
            _ => {
                if let Some(rest) = s.strip_prefix("pull:") {
                    offset(rest).map(|(x, y)| Self::Pull(x, y))
                } else if let Some(rest) = s.strip_prefix("menu:") {
                    offset(rest).map(|(x, y)| Self::Menu(x, y))
                } else if let Some(rest) = s.strip_prefix("pick:") {
                    rest.trim().parse().ok().map(Self::Pick)
                } else if let Some(rest) = s.strip_prefix("wait:") {
                    rest.trim().parse().ok().map(Self::Wait)
                } else if let Some(rest) = s.strip_prefix("floor:") {
                    offset(rest).map(|(x, y)| Self::Floor(x, y))
                } else {
                    offset(s).map(|(x, y)| Self::Tile(x, y))
                }
            }
        }
    }
}

/// `--use`: the steps still to do.
#[derive(Resource)]
struct ScriptedUse(VecDeque<UseStep>);

#[derive(Resource)]
struct ScreenshotPlan(Option<String>, f32);

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

#[derive(Component)]
struct StatusLine;

/// Our own health, state and intent, as last reported by the server.
#[derive(Resource, Default)]
struct OwnStatus(Option<StatusUpdate>);

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

/// The camera that draws the map, in the square on the left.
#[derive(Component)]
struct WorldCamera;

/// The HUD hover tip in the stat panel.
#[derive(Component)]
struct TipLine;

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
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: format!("rust-station13 (protocol {})", rs13_network::PROTOCOL_VERSION),
                        resolution: args.size.map_or_else(default, |(w, h)| {
                            bevy::window::WindowResolution::new(w as u32, h as u32)
                        }),
                        ..default()
                    }),
                    ..default()
                })
                // Crisp pixel art.
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins((
            ClientNetworkPlugin,
            audio::SoundPlugin,
            lighting::LightingPlugin,
            scaling::ScalingPlugin,
            shadows::ShadowPlugin,
            fov::FovPlugin,
            glow::GlowPlugin,
            depth::DepthPlugin,
            sprite_shadows::SpriteShadowsPlugin,
            gas::GasPlugin,
            heat::HeatPlugin,
            world_view::WorldViewPlugin,
            space::SpacePlugin,
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(ServerAddr(args.server))
        .insert_resource(WantedName(args.name))
        .insert_resource(ScreenshotPlan(args.screenshot, args.shot_after))
        .insert_resource(ScriptedUse(args.use_at.into()))
        .init_resource::<ChatInput>()
        .init_resource::<OwnStatus>()
        .init_resource::<hud::WalkMode>()
        .init_resource::<hud::HudTip>()
        .init_resource::<wire_panel::OpenPanel>()
        .init_resource::<alarm_panel::OpenAlarm>()
        .init_resource::<context_menu::MenuRequest>()
        .init_resource::<apc_panel::OpenApc>()
        .init_resource::<monitor_panel::OpenMonitor>()
        .init_resource::<laser_panel::OpenLaser>()
        .init_resource::<freezer_panel::OpenFreezer>()
        .init_resource::<smes_panel::OpenSmes>()
        .init_resource::<storage_panel::OpenStorage>()
        .init_resource::<OverlayOn>()
        .init_resource::<TileSprites>()
        .insert_resource(Loc(localizer))
        .init_resource::<MessageLog>()
        .add_systems(
            Startup,
            (
                (sprites::load_atlas, hud::spawn_hud).chain(),
                (setup, scaling::spawn_world_view).chain(),
                spawn_panel,
            ),
        )
        .add_systems(
            Update,
            (
                draw_map,
                add_sprites,
                update_appearance,
                sprites::join_lines,
                show_only_placed,
                mark_own_player,
                send_name,
                chat_typing,
                (
                    send_move_input,
                    send_hotkeys,
                    send_clicks,
                    context_menu::open_menu,
                    context_menu::close_menu,
                )
                    .chain(),
                receive_notices,
                receive_status,
                // Machine windows and the open container.
                (
                    (wire_panel::receive_panel, wire_panel::show_panel).chain(),
                    (alarm_panel::receive_alarm, alarm_panel::show_alarm).chain(),
                    (apc_panel::receive_apc, apc_panel::show_apc).chain(),
                    (monitor_panel::receive_monitor, monitor_panel::show_monitor).chain(),
                    (laser_panel::receive_laser, laser_panel::show_laser).chain(),
                    (freezer_panel::receive_freezer, freezer_panel::show_freezer).chain(),
                    (smes_panel::receive_smes, smes_panel::show_smes).chain(),
                    (storage_panel::receive_storage, storage_panel::show_storage).chain(),
                ),
                glide,
                (
                    apply_facing,
                    sprites::wear_layers,
                    sprites::overlays,
                    sprites::dress,
                    sprites::animate,
                )
                    .chain(),
                follow_own_player,
                (
                    hud::update_hud,
                    update_tip_line,
                    update_log,
                    update_chat_line,
                    update_status_line,
                    spawn_overlay,
                    update_overlay,
                    show_own_deck,
                    take_screenshot,
                    scripted_use,
                )
                    .chain(),
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
        screenshot: None,
        shot_after: 4.0,
        size: None,
        use_at: Vec::new(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--server" => {
                if let Some(addr) = it.next().and_then(|s| s.parse().ok()) {
                    args.server = addr;
                }
            }
            "--lang" => args.russian = it.next().as_deref() == Some("ru"),
            "--name" => args.name = it.next(),
            "--screenshot" => args.screenshot = it.next(),
            "--shot-after" => {
                if let Some(secs) = it.next().and_then(|s| s.parse().ok()) {
                    args.shot_after = secs;
                }
            }
            "--use" => args.use_at.extend(it.next().as_deref().and_then(UseStep::parse)),
            "--size" => {
                args.size = it.next().and_then(|s| {
                    let (w, h) = s.split_once('x')?;
                    Some((w.parse().ok()?, h.parse().ok()?))
                })
            }
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
    let screen = hud::SCREEN_CELLS * TILE_PX;
    commands.spawn((
        Camera2d,
        // Draws into the world image before the window is drawn (`scaling.rs`).
        Camera {
            order: -1,
            ..default()
        },
        // What gives off light glows (`glow.rs`): only what is drawn brighter than white
        // spreads, and colours are kept as drawn.
        bevy::render::view::Tonemapping::None,
        bevy::post_process::bloom::Bloom {
            intensity: 0.3,
            low_frequency_boost: 0.0,
            high_pass_frequency: 0.6,
            prefilter: bevy::post_process::bloom::BloomPrefilter {
                threshold: 1.0,
                threshold_softness: 0.2,
            },
            composite_mode: bevy::post_process::bloom::BloomCompositeMode::Additive,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::Fixed {
                width: screen,
                height: screen,
            },
            ..OrthographicProjection::default_2d()
        }),
        WorldCamera,
    ));
    // The window's camera: it draws the UI, which includes the world image, the HUD and the
    // text panel. It draws no world itself: nothing is on its layer.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        IsDefaultUiCamera,
        bevy::camera::visibility::RenderLayers::layer(31),
    ));
    connect(&mut commands, server.0, unique_client_id())?;
    info!(server = %server.0, "connecting");
    Ok(())
}

/// The text panel right of the map, as in the classic BYOND client: stats on top, the
/// message output below, the say line at the bottom.
fn spawn_panel(mut commands: Commands) {
    let grey = Color::srgb(0.83, 0.82, 0.78);
    let edge = Color::srgb(0.5, 0.5, 0.5);
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::VMin(100.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(4.0)),
            row_gap: Val::Px(4.0),
            ..default()
        },
        BackgroundColor(grey),
        children![
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                BorderColor::all(edge),
                children![
                    (
                        StatusLine,
                        Text::new(""),
                        ui_text_font(14.0),
                        TextColor(Color::BLACK)
                    ),
                    (
                        TipLine,
                        Text::new(" "),
                        ui_text_font(14.0),
                        TextColor(Color::srgb(0.1, 0.1, 0.45))
                    ),
                ],
            ),
            (
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    overflow: Overflow::clip(),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                BorderColor::all(edge),
                children![(
                    LogText,
                    Text::new(""),
                    ui_text_font(14.0),
                    TextColor(Color::BLACK)
                )],
            ),
            (
                Node {
                    min_height: Val::Px(22.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::horizontal(Val::Px(4.0)),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                BorderColor::all(edge),
                children![(
                    ChatLine,
                    Text::new(""),
                    ui_text_font(14.0),
                    TextColor(Color::BLACK)
                )],
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
        ("human", "unconscious") => (Color::srgb(0.45, 0.45, 0.55), TILE_PX * 0.7, 3.0),
        ("human", "dead") => (Color::srgb(0.45, 0.10, 0.10), TILE_PX * 0.7, 3.0),
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
        ("pipe", _) => (Color::srgb(0.35, 0.40, 0.55), 8.0, 0.6),
        // Under cables and gas pipes (Luna: disposal pipes 2.3, gas pipes 2.4).
        ("disposal-pipe", _) => (Color::srgb(0.45, 0.45, 0.45), 10.0, 0.5),
        ("cable", _) => (Color::srgb(0.85, 0.20, 0.20), 5.0, 0.55),
        ("smes", _) => (Color::srgb(0.85, 0.80, 0.30), TILE_PX - 4.0, 1.5),
        ("apc", "off") => (Color::srgb(0.30, 0.30, 0.30), 18.0, 1.6),
        ("apc", "low") => (Color::srgb(0.90, 0.50, 0.15), 18.0, 1.6),
        ("apc", _) => (Color::srgb(0.25, 0.80, 0.30), 18.0, 1.6),
        (tank, _) if is_pipe_tank(tank) => (Color::srgb(0.30, 0.45, 0.75), TILE_PX - 4.0, 1.5),
        (canister, _) if is_canister(canister) => (Color::srgb(0.35, 0.55, 0.85), 20.0, 1.5),
        ("pump" | "vent" | "scrubber", "off") => (Color::srgb(0.30, 0.30, 0.30), 16.0, 0.7),
        ("pump", _) => (Color::srgb(0.90, 0.85, 0.40), 16.0, 0.7),
        ("vent", _) => (Color::srgb(0.70, 0.80, 0.90), 16.0, 0.7),
        ("scrubber", _) => (Color::srgb(0.55, 0.85, 0.60), 16.0, 0.7),
        ("valve", "open") => (Color::srgb(0.40, 0.80, 0.40), 12.0, 0.7),
        ("valve", _) => (Color::srgb(0.80, 0.30, 0.30), 12.0, 0.7),
        ("connector", _) => (Color::srgb(0.50, 0.50, 0.50), 14.0, 0.65),
        ("meter", _) => (Color::srgb(0.85, 0.85, 0.85), 10.0, 0.75),
        ("laser-beam", _) => (Color::srgb(0.4, 0.4, 1.0), TILE_PX, 3.1),
        // Burning air, over everything but the walls' fixtures.
        ("fire", _) => (Color::srgb(1.0, 0.5, 0.1), TILE_PX, 3.2),
        // Blast doors stand like airlocks.
        ("blast-door", _) => (Color::srgb(0.35, 0.35, 0.40), TILE_PX - 1.0, 2.0),
        // Flying sparks: over the fire, under the heat haze.
        ("sparks", _) => (Color::srgb(1.0, 0.9, 0.4), 8.0, 3.25),
        // A chair facing north has its back to us: drawn over its sitter and their clothes
        // (Luna `handle_rotation`, `FLY_LAYER`).
        (chair, "north") if rs13_classic::chairs::is_chair(chair) => {
            (Color::srgb(0.55, 0.35, 0.20), TILE_PX - 6.0, 3.15)
        }
        // On the wall, above people.
        ("light" | "light-small", _) => (Color::srgb(0.95, 0.95, 0.75), 10.0, 3.5),
        ("light-switch", _) => (Color::srgb(0.80, 0.80, 0.70), 8.0, 3.5),
        // Over the airlock it shares a tile with.
        ("firedoor", state) if state.starts_with("door0") => (Color::srgb(0.85, 0.30, 0.20), 6.0, 2.1),
        ("firedoor", _) => (Color::srgb(0.85, 0.30, 0.20), TILE_PX - 2.0, 2.1),
        // Under what is put on them.
        (closet, _) if rs13_classic::closets::is_closet(closet) => {
            (Color::srgb(0.45, 0.50, 0.55), TILE_PX - 2.0, 0.9)
        }
        ("table" | "table-reinforced" | "table-wood" | "rack", _) => {
            (Color::srgb(0.55, 0.55, 0.60), TILE_PX - 2.0, 0.9)
        }
        _ => (Color::srgb(1.0, 0.0, 1.0), 14.0, 1.0),
    }
}

/// The topmost visible sprite with a non-transparent pixel at `world`. Fire and beams can't
/// be clicked (BYOND `mouse_opacity = 0`).
fn pick_sprite(
    world: Vec2,
    drawn: &Query<(&NetId, &Appearance, &GlobalTransform, &Sprite, &Visibility), Without<Hidden>>,
    atlas: &SpriteAtlas,
) -> Option<NetId> {
    let half = TILE_PX / 2.0;
    drawn
        .iter()
        .filter(|(_, a, _, _, v)| {
            **v != Visibility::Hidden && !matches!(a.sprite.as_str(), "fire" | "laser-beam" | "sparks")
        })
        .filter_map(|(id, _, at, sprite, _)| {
            let centre = at.translation();
            let (x, y) = (world.x - (centre.x - half), (centre.y + half) - world.y);
            if !(0.0..TILE_PX).contains(&x) || !(0.0..TILE_PX).contains(&y) {
                return None;
            }
            let index = sprite.texture_atlas.as_ref()?.index;
            atlas
                .opaque_at(index, x as u32, y as u32)
                .then_some((*id, centre.z))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// Which thing on a tile a click picks: lowest rank wins.
fn click_rank(appearance: &Appearance) -> u8 {
    match appearance.sprite.as_str() {
        "human" => 3,
        "airlock" | "table" | "table-reinforced" | "table-wood" | "rack" => 2,
        chair if rs13_classic::chairs::is_chair(chair) => 2,
        closet if rs13_classic::closets::is_closet(closet) => 2,
        // A closed or moving firedoor is what gets clicked; an open one is out of the way.
        "firedoor" if appearance.state.starts_with("door0") => 5,
        "firedoor" => 1,
        "pipe" | "cable" => 4,
        "pump" | "vent" | "scrubber" | "valve" | "connector" | "meter" | "smes" | "apc" | "light"
        | "light-small" | "light-switch" => 1,
        machine if is_pipe_tank(machine) || is_canister(machine) => 1,
        _ => 0,
    }
}

/// The sprites drawn for each tile, and the tile they show.
#[derive(Resource, Default)]
struct TileSprites(HashMap<TilePos, (String, Vec<Entity>)>);

/// The facing of Luna's window sprite that fills the whole tile (south-east; the straight
/// facings are panes along one edge).
const FULL_TILE_GLASS: u32 = 4;

/// Draw the map's tiles when it arrives, and redraw those that change (floor tiles pried
/// up or put back): Luna sprites, or flat squares as a fallback.
fn draw_map(
    mut commands: Commands,
    atlas: Res<SpriteAtlas>,
    maps: Query<&MapLayer, Changed<MapLayer>>,
    mut drawn: ResMut<TileSprites>,
) {
    for MapLayer(map) in &maps {
        if drawn.0.is_empty() {
            info!(width = map.width, height = map.height, "map received");
        }
        for y in 0..map.height as i32 {
            for x in 0..map.width as i32 {
                let pos = TilePos::new(x, y, map.z);
                let Some(tile) = map.tile(pos) else { continue };
                if drawn.0.get(&pos).is_some_and(|(id, _)| *id == tile.id) {
                    continue;
                }
                if let Some((_, old)) = drawn.0.remove(&pos) {
                    for sprite in old {
                        commands.entity(sprite).despawn();
                    }
                }
                let mut sprites = Vec::new();
                let (sprite, state) = match tile.id.as_str() {
                    "wall" => ("tile:wall", sprites::wall_state(map, pos)),
                    "r-wall" => ("tile:r-wall", sprites::wall_state(map, pos)),
                    "space" => ("tile:space", sprites::space_state(pos)),
                    other => (
                        if other == "floor" {
                            "tile:floor"
                        } else {
                            "tile:plating"
                        },
                        String::new(),
                    ),
                };
                let transform = Transform::from_translation(world_pos(pos, 0.0));
                // A grille over the plating, and in a window tile glass filling the tile:
                // Luna's window sprite facing a diagonal, the same from every side (its
                // straight facings are thin panes along one edge).
                let parts: &[(&str, u32)] = match tile.id.as_str() {
                    "window" => &[("tile:grille", 0), ("tile:window", FULL_TILE_GLASS)],
                    "grille" => &[("tile:grille", 0)],
                    _ => &[],
                };
                {
                    for (layer, &(part, facing)) in parts.iter().enumerate() {
                        if let Some(image) = atlas.sprite(part, "", facing) {
                            let z = 0.001 * (layer + 1) as f32;
                            sprites.push(
                                commands
                                    .spawn((
                                        image,
                                        Transform::from_translation(world_pos(pos, z)),
                                        OnDeck(map.z),
                                    ))
                                    .id(),
                            );
                        }
                    }
                }
                // Space is drawn under the map by `space.rs`; its tiles draw nothing.
                if tile.id == "space" {
                    drawn.0.insert(pos, (tile.id.clone(), Vec::new()));
                    continue;
                }
                sprites.push(match atlas.sprite(sprite, &state, 0) {
                    Some(image) => commands.spawn((image, transform, OnDeck(map.z))).id(),
                    None => commands
                        .spawn((
                            Sprite::from_color(tile_color(&tile.id), Vec2::splat(TILE_PX - 1.0)),
                            transform,
                            OnDeck(map.z),
                        ))
                        .id(),
                });
                drawn.0.insert(pos, (tile.id.clone(), sprites));
            }
        }
    }
}

/// Give new objects their look: an animated Luna sprite, or a flat square if the atlas has
/// nothing for them. People's clothes are layered on by `sprites::wear_layers`.
fn add_sprites(
    mut commands: Commands,
    time: Res<Time>,
    atlas: Res<SpriteAtlas>,
    objects: Query<(Entity, &Appearance, Option<&GridPos>), Added<Appearance>>,
) {
    for (entity, appearance, pos) in &objects {
        let (color, size, z) = object_look(appearance);
        let pos = pos.map(|p| p.0).unwrap_or(TilePos::new(0, 0, 0));
        let transform = Transform::from_translation(world_pos(pos, z));
        let state = sprites::state_for(appearance);
        let Some(image) = atlas.sprite(&appearance.sprite, &state, 0) else {
            commands
                .entity(entity)
                .insert((Sprite::from_color(color, Vec2::splat(size)), transform));
            continue;
        };
        let look = Look {
            sprite: appearance.sprite.clone(),
            state: state.clone(),
            facing: 0,
            since: time.elapsed_secs(),
        };
        commands.entity(entity).insert((image, transform, look));
    }
}

/// Things placed facing a way show that direction of their sprite (BYOND `dir`).
fn apply_facing(
    mut objects: Query<(&rs13_world::Facing, &mut Look), Or<(Changed<rs13_world::Facing>, Added<Look>)>>,
) {
    for (facing, mut look) in &mut objects {
        let facing = u32::from(facing.0);
        if look.facing != facing {
            look.facing = facing;
        }
    }
}

fn update_appearance(
    time: Res<Time>,
    mut objects: Query<
        (
            &Appearance,
            &mut Sprite,
            &mut Transform,
            Option<&mut Look>,
            Has<OwnPlayer>,
        ),
        Changed<Appearance>,
    >,
) {
    for (appearance, mut sprite, mut transform, look, own) in &mut objects {
        // Some things change layer with their state (a chair turned to face north).
        let z = object_look(appearance).2;
        if transform.translation.z != z {
            transform.translation.z = z;
        }
        match look {
            Some(mut look) => {
                let state = sprites::state_for(appearance);
                if look.state != state {
                    look.state = state;
                    look.since = time.elapsed_secs();
                }
            }
            // Flat squares: our own player stays green while it is up and about.
            None => {
                sprite.color = if own && appearance.state.is_empty() {
                    OWN_PLAYER_COLOR
                } else {
                    object_look(appearance).0
                };
            }
        }
    }
}

/// Draw what lies on the map in sight: not held items (they have no `GridPos`), not things
/// under floor tiles, not what the player can't see (`shadows::Sight`), and not other
/// people behind the player's back (`fov`). A closed door's tile is seen like a wall's
/// (`shadows::seen_tiles`).
fn show_only_placed(
    sight: Res<shadows::Sight>,
    fov: Res<fov::Fov>,
    mut objects: Query<
        (
            &mut Visibility,
            Option<&GridPos>,
            Has<Hidden>,
            Has<Player>,
            Has<OwnPlayer>,
            Has<rs13_world::Silhouette>,
        ),
        With<Appearance>,
    >,
) {
    for (mut visibility, placed, hidden, player, own, shape) in &mut objects {
        let person = player || shape;
        let seen = placed.is_some_and(|p| sight.sees(p.0) && !(person && !own && fov.hides(p.0)));
        visibility.set_if_neq(if seen && !hidden {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

fn mark_own_player(
    mut commands: Commands,
    mut own: Query<(Entity, &mut Sprite, Has<Look>), (Added<Controlled>, With<Player>)>,
) {
    for (entity, mut sprite, has_look) in &mut own {
        if !has_look {
            sprite.color = OWN_PLAYER_COLOR;
        }
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
    walk: Res<hud::WalkMode>,
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
    // With Ctrl held, directions turn on the spot instead (`send_hotkeys`).
    let input = MoveInput {
        dir: Dir::from_offset(dx, dy).filter(|_| turns(&keys).is_none()),
        // Shift does the opposite of the move-intent button while held.
        run: walk.0 == keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
    };
    if *last_sent != Some(input) {
        sender.send::<IntentChannel>(input);
        *last_sent = Some(input);
    }
}

/// With Ctrl held, directions turn on the spot instead of stepping: the ways pressed this
/// frame, to turn to. `None` without Ctrl.
fn turns(keys: &ButtonInput<KeyCode>) -> Option<Vec<Dir>> {
    if !keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        return None;
    }
    let directions = [
        (KeyCode::KeyW, KeyCode::ArrowUp, Dir::North),
        (KeyCode::KeyS, KeyCode::ArrowDown, Dir::South),
        (KeyCode::KeyD, KeyCode::ArrowRight, Dir::East),
        (KeyCode::KeyA, KeyCode::ArrowLeft, Dir::West),
    ];
    Some(
        directions
            .into_iter()
            .filter(|(a, b, _)| keys.any_just_pressed([*a, *b]))
            .map(|(.., dir)| dir)
            .collect(),
    )
}

fn send_hotkeys(
    chat: Res<ChatInput>,
    keys: Res<ButtonInput<KeyCode>>,
    own: Query<&HandsView, With<OwnPlayer>>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    // `chat` may have just closed this frame on Enter; only a closed line takes hotkeys.
    if chat.open || chat.is_changed() {
        return;
    }
    if let Some(turns) = turns(&keys) {
        for dir in turns {
            sender.send::<IntentChannel>(ActionInput::Face(dir));
        }
        return;
    }
    if keys.just_pressed(KeyCode::KeyQ) {
        sender.send::<IntentChannel>(ActionInput::Drop);
    }
    if keys.just_pressed(KeyCode::KeyX) {
        sender.send::<IntentChannel>(ActionInput::SwapHands);
    }
    if keys.just_pressed(KeyCode::KeyE) {
        sender.send::<IntentChannel>(ActionInput::ToggleInternals);
    }
    // Use the item in the active hand (Luna `attack_self`).
    if keys.just_pressed(KeyCode::KeyZ)
        && let Ok(hands) = own.single()
        && let Some(item) = match hands.active {
            Hand::Left => hands.left,
            Hand::Right => hands.right,
        }
    {
        sender.send::<IntentChannel>(ActionInput::UseOn(item));
    }
    // Classic hotkeys: 1 help, 4 harm (2 disarm and 3 grab come later).
    if keys.just_pressed(KeyCode::Digit1) {
        sender.send::<IntentChannel>(ActionInput::SetIntent(Intent::Help));
    }
    if keys.just_pressed(KeyCode::Digit4) {
        sender.send::<IntentChannel>(ActionInput::SetIntent(Intent::Harm));
    }
}

/// Where the cursor is in the world, if it is over the map square.
fn cursor_world(window: &Window, camera: &GlobalTransform) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    // Right of the map square is the text panel.
    let square = window.width().min(window.height());
    if cursor.x > square || square <= 0.0 {
        return None;
    }
    // The square shows the camera's 16 cells, centred on the camera (`scaling.rs`).
    let centre = camera.translation().truncate();
    Some(centre + Vec2::new(cursor.x / square - 0.5, 0.5 - cursor.y / square) * scaling::SCREEN_PX)
}

/// The tile under the cursor, if it is over the map square.
fn cursor_tile(window: &Window, camera: &GlobalTransform) -> Option<(i32, i32)> {
    let world = cursor_world(window, camera)?;
    Some((
        (world.x / TILE_PX).round() as i32,
        (world.y / TILE_PX).round() as i32,
    ))
}

/// Left click: use the active hand on the topmost thing on the clicked tile, on our own
/// deck. Shift+click: examine it. The server checks reach. A click that closes the
/// right-click menu does nothing else.
#[allow(clippy::too_many_arguments)]
fn send_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<&GlobalTransform, With<WorldCamera>>,
    objects: Query<(&NetId, &GridPos, &Appearance), Without<Hidden>>,
    drawn: Query<(&NetId, &Appearance, &GlobalTransform, &Sprite, &Visibility), Without<Hidden>>,
    atlas: Res<SpriteAtlas>,
    own: Query<&GridPos, With<OwnPlayer>>,
    menus: Query<(), With<context_menu::ContextMenu>>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
    hud: Query<&bevy::picking::hover::Hovered>,
) {
    // A click on the HUD is not a click on the world under it.
    if !mouse.just_pressed(MouseButton::Left) || hud.iter().any(|h| h.0) || !menus.is_empty() {
        return;
    }
    let (Ok(mut sender), Ok(me)) = (sender.single_mut(), own.single()) else {
        return;
    };
    let Some(world) = cursor_world(&window, &camera) else {
        return;
    };
    let tile = (
        (world.x / TILE_PX).round() as i32,
        (world.y / TILE_PX).round() as i32,
    );

    // What is drawn on top under the cursor, where its sprite isn't see-through (BYOND's
    // click rule); failing that, the tile's things by rank: items first, then machines,
    // doors and tables, people, and bare pipes last.
    let target = pick_sprite(world, &drawn, &atlas).or_else(|| {
        objects
            .iter()
            .filter(|(_, pos, a)| {
                (pos.0.x, pos.0.y, pos.0.z) == (tile.0, tile.1, me.0.z)
                    && !matches!(a.sprite.as_str(), "fire" | "laser-beam" | "sparks")
            })
            .min_by_key(|(_, _, a)| click_rank(a))
            .map(|(id, ..)| *id)
    });
    let examine = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    // Nothing there: click the tile itself (to lay cable, pry up floor tiles).
    let Some(target) = target else {
        if !examine {
            sender.send::<IntentChannel>(ActionInput::UseOnTile(TilePos::new(tile.0, tile.1, me.0.z)));
        }
        return;
    };
    // Ctrl-click pulls it along (Luna `CtrlClick`).
    let pull = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    sender.send::<IntentChannel>(if examine {
        ActionInput::Examine(target)
    } else if pull {
        ActionInput::Pull(target)
    } else {
        ActionInput::UseOn(target)
    });
}

fn receive_status(
    mut own: ResMut<OwnStatus>,
    mut receivers: Query<&mut MessageReceiver<StatusUpdate>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(status) = receiver.receive().last() {
            own.0 = Some(status);
        }
    }
}

fn update_status_line(loc: Res<Loc>, own: Res<OwnStatus>, mut text: Single<&mut Text, With<StatusLine>>) {
    if !own.is_changed() {
        return;
    }
    let Some(status) = &own.0 else { return };
    let health = match status.stat {
        MobStat::Dead => loc.0.format("hud-health-dead", None),
        MobStat::Unconscious => loc.0.format("hud-health-crit", None),
        MobStat::Conscious => {
            let mut args = rs13_i18n::Args::new();
            args.set("value", status.health);
            loc.0.format("hud-health", Some(&args))
        }
    }
    .unwrap_or_default();
    let mut args = rs13_i18n::Args::new();
    args.set(
        "intent",
        match status.intent {
            Intent::Help => "help",
            Intent::Harm => "harm",
        },
    );
    let intent = loc.0.format("hud-intent", Some(&args)).unwrap_or_default();
    let mut args = rs13_i18n::Args::new();
    args.set("value", f64::from(status.pressure_dkpa) / 10.0);
    let pressure = loc.0.format("hud-pressure", Some(&args)).unwrap_or_default();
    let alerts: Vec<String> = status
        .alerts
        .iter()
        .map(|a| {
            let id = match a {
                EnvAlert::Vacuum => "alert-vacuum",
                EnvAlert::LowPressure => "alert-low-pressure",
                EnvAlert::HighPressure => "alert-high-pressure",
                EnvAlert::LowOxygen => "alert-low-oxygen",
                EnvAlert::Cold => "alert-cold",
                EnvAlert::Hot => "alert-hot",
            };
            loc.0.format(id, None).unwrap_or_default()
        })
        .collect();
    text.0 = format!("{health}    {intent}    {pressure}");
    if !alerts.is_empty() {
        text.0.push_str(&format!("    ⚠ {}", alerts.join(" · ")));
    }
}

/// One tinted square per tile of a deck (its `z`, the tile's index), shown with F3: the
/// atmos debug overlay.
#[derive(Component)]
struct OverlayTile(i32, usize);

/// On what is drawn for one deck (its tiles, their shading): shown only while our player
/// is on that deck.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnDeck(pub i32);

/// Show what belongs to our player's deck and hide the other decks.
fn show_own_deck(
    own: Query<&GridPos, With<OwnPlayer>>,
    mut drawn: Query<(&OnDeck, &mut Visibility)>,
    mut last: Local<Option<i32>>,
    added: Query<(), Added<OnDeck>>,
) {
    let Some(deck) = own.iter().next().map(|p| p.0.z) else {
        return;
    };
    if *last == Some(deck) && added.is_empty() {
        return;
    }
    *last = Some(deck);
    for (on, mut visibility) in &mut drawn {
        visibility.set_if_neq(if on.0 == deck {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

#[derive(Resource, Default)]
struct OverlayOn(bool);

fn spawn_overlay(mut commands: Commands, maps: Query<&MapLayer, Added<MapLayer>>) {
    for MapLayer(map) in &maps {
        for y in 0..map.height as i32 {
            for x in 0..map.width as i32 {
                let index = (y * map.width as i32 + x) as usize;
                commands.spawn((
                    OverlayTile(map.z, index),
                    Sprite::from_color(Color::NONE, Vec2::splat(TILE_PX - 1.0)),
                    Transform::from_translation(world_pos(TilePos::new(x, y, map.z), 0.5)),
                    Visibility::Hidden,
                ));
            }
        }
    }
}

/// Pressure colour: blue above one atmosphere, green at it, through yellow to red at vacuum.
fn pressure_color(dkpa: u16) -> Color {
    let atm = f32::from(dkpa) / 1013.25;
    if atm > 1.05 {
        Color::srgba(0.2, 0.4, 1.0, 0.45)
    } else {
        let t = atm.clamp(0.0, 1.0);
        Color::srgba(1.0 - t, t, 0.1, 0.45)
    }
}

fn update_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    chat: Res<ChatInput>,
    mut on: ResMut<OverlayOn>,
    views: Query<(&MapLayer, &AtmosView)>,
    own: Query<&GridPos, With<OwnPlayer>>,
    mut tiles: Query<(&OverlayTile, &mut Sprite, &mut Visibility)>,
) {
    if !chat.open && keys.just_pressed(KeyCode::F3) {
        on.0 = !on.0;
    }
    // Our deck's air picture.
    let deck = own.iter().next().map(|p| p.0.z);
    let Some(view) = views.iter().find(|(m, _)| Some(m.0.z) == deck).map(|(_, v)| v) else {
        return;
    };
    for (OverlayTile(z, i), mut sprite, mut visibility) in &mut tiles {
        let on = on.0 && Some(*z) == deck;
        visibility.set_if_neq(if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if on && let Some(p) = view.pressure_dkpa.get(*i) {
            sprite.color = pressure_color(*p);
        }
    }
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

/// Where a gliding mob really is; its `Transform` is this snapped to the world image's
/// pixels (`scaling.rs`).
#[derive(Component)]
struct Glided(Vec3);

/// Slide mobs towards their server position, like BYOND's glide; other objects snap.
fn glide(
    mut commands: Commands,
    time: Res<Time>,
    scale: Res<scaling::PixelScale>,
    mut movers: Query<(
        Entity,
        &GridPos,
        &mut Transform,
        Has<Player>,
        Has<rs13_world::Silhouette>,
        Option<&mut Glided>,
        Option<&rs13_world::PixelOffset>,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, pos, mut transform, player, shape, glided, offset) in &mut movers {
        // Someone in the dark glides like anyone else (`sprite_shadows.rs`, Silhouettes).
        let is_player = player || shape;
        // Hung on a wall: drawn shifted onto it (BYOND `pixel_x`, `pixel_y`).
        let shift = offset.map_or(Vec3::ZERO, |o| {
            Vec3::new(f32::from(o.0[0]), f32::from(o.0[1]), 0.0)
        });
        let target = world_pos(pos.0, transform.translation.z) + shift;
        let at = glided.as_ref().map_or(transform.translation, |g| g.0);
        let offset = target - at;
        // Teleports and large corrections snap instead of sliding across the map.
        if !is_player || offset.length() > TILE_PX * 2.0 {
            transform.translation = target;
            if glided.is_some() {
                commands.entity(entity).remove::<Glided>();
            }
            continue;
        }
        // Glide at least at walking pace, faster the further behind: running in shoes can
        // outpace a fixed glide, which then falls behind and jumps.
        let speed = GLIDE_TILES_PER_SEC + 8.0 * offset.length() / TILE_PX;
        let at = at + offset.clamp_length_max(speed * TILE_PX * dt);
        let snapped = Vec3::new(scale.snap(at.x), scale.snap(at.y), at.z);
        if transform.translation != snapped {
            transform.translation = snapped;
        }
        match glided {
            Some(mut g) => g.0 = at,
            None => {
                commands.entity(entity).insert(Glided(at));
            }
        }
    }
}

fn follow_own_player(
    own: Query<&Transform, (With<OwnPlayer>, Without<WorldCamera>)>,
    mut camera: Single<&mut Transform, With<WorldCamera>>,
) {
    // The player stands in the middle of the 15 x 15 view: half a cell left of the
    // screen's centre (the HUD column is on the right) and half a cell above it (the
    // HUD row is below). The player's position is already on the world image's pixels.
    if let Ok(player) = own.single() {
        camera.translation.x = player.translation.x + TILE_PX / 2.0;
        camera.translation.y = player.translation.y - TILE_PX / 2.0;
    }
}

fn update_tip_line(tip: Res<hud::HudTip>, mut text: Single<&mut Text, With<TipLine>>) {
    if tip.is_changed() {
        // A space keeps the line's height when there is no tip.
        text.0 = if tip.0.is_empty() {
            " ".to_owned()
        } else {
            tip.0.clone()
        };
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

/// With `--use`: from two seconds after our player appears, do each step half a second
/// apart, as the clicks or keys would.
fn scripted_use(
    time: Res<Time>,
    mut plan: ResMut<ScriptedUse>,
    own: Query<(&GridPos, &HandsView), With<OwnPlayer>>,
    objects: Query<(&NetId, &GridPos, &Appearance), Without<Hidden>>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
    mut menu: ResMut<context_menu::MenuRequest>,
    mut next_at: Local<Option<f32>>,
) {
    let (Some(step), Ok((me, hands))) = (plan.0.front().copied(), own.single()) else {
        return;
    };
    let now = time.elapsed_secs();
    if now < *next_at.get_or_insert(now + 2.0) {
        return;
    }
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    if let UseStep::Wait(seconds) = step {
        plan.0.pop_front();
        *next_at = Some(now + seconds);
        return;
    }
    match step {
        UseStep::Menu(dx, dy) => menu.open_at = Some((dx, dy)),
        UseStep::Pick(n) => menu.pick = Some(n),
        _ => {}
    }
    let action = match step {
        UseStep::Wait(_) | UseStep::Menu(..) | UseStep::Pick(_) => None,
        UseStep::Tile(dx, dy) | UseStep::Pull(dx, dy) => {
            let tile = (me.0.x + dx, me.0.y + dy);
            objects
                .iter()
                .filter(|(_, pos, a)| {
                    (pos.0.x, pos.0.y, pos.0.z) == (tile.0, tile.1, me.0.z)
                        && !matches!(a.sprite.as_str(), "fire" | "laser-beam" | "sparks")
                })
                .min_by_key(|(_, _, a)| click_rank(a))
                .map(|(id, ..)| match step {
                    UseStep::Pull(..) => ActionInput::Pull(*id),
                    _ => ActionInput::UseOn(*id),
                })
        }
        UseStep::Swap => Some(ActionInput::SwapHands),
        UseStep::Held => match hands.active {
            Hand::Right => hands.left,
            Hand::Left => hands.right,
        }
        .map(ActionInput::UseOn),
        UseStep::InHand => match hands.active {
            Hand::Left => hands.left,
            Hand::Right => hands.right,
        }
        .map(ActionInput::UseOn),
        UseStep::Floor(dx, dy) => Some(ActionInput::UseOnTile(TilePos::new(
            me.0.x + dx,
            me.0.y + dy,
            me.0.z,
        ))),
    };
    if let Some(action) = action {
        sender.send::<IntentChannel>(action);
    }
    plan.0.pop_front();
    *next_at = Some(now + 0.5);
}

/// With `--screenshot PATH`: four seconds (or `--shot-after`) after our player appears, save
/// the window to PATH; a second later, quit.
fn take_screenshot(
    mut commands: Commands,
    time: Res<Time>,
    mut plan: ResMut<ScreenshotPlan>,
    own: Query<(), With<OwnPlayer>>,
    mut joined_at: Local<Option<f32>>,
    mut taken_at: Local<Option<f32>>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    if let Some(at) = *taken_at {
        if now > at + 1.0 {
            exit.write(AppExit::Success);
        }
        return;
    }
    if plan.0.is_none() || own.is_empty() {
        return;
    }
    let joined = *joined_at.get_or_insert(now);
    if now > joined + plan.1
        && let Some(path) = plan.0.take()
    {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(path));
        *taken_at = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_and_a_direction_turns_instead_of_stepping() {
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::ArrowUp);
        assert_eq!(turns(&keys), None, "without Ctrl, arrows step");
        keys.clear();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::KeyA);
        assert_eq!(turns(&keys), Some(vec![Dir::West]));
        // Held on, it doesn't turn again, but still doesn't step.
        keys.clear();
        assert_eq!(turns(&keys), Some(vec![]));
        keys.release(KeyCode::ControlLeft);
        assert_eq!(turns(&keys), None);
    }

    #[test]
    fn use_steps_parse() {
        assert_eq!(UseStep::parse("1,-2"), Some(UseStep::Tile(1, -2)));
        assert_eq!(UseStep::parse("swap"), Some(UseStep::Swap));
        assert_eq!(UseStep::parse("held"), Some(UseStep::Held));
        assert_eq!(UseStep::parse("self"), Some(UseStep::InHand));
        assert_eq!(UseStep::parse("floor:3,0"), Some(UseStep::Floor(3, 0)));
        assert_eq!(UseStep::parse("wait:2.5"), Some(UseStep::Wait(2.5)));
        assert_eq!(UseStep::parse("menu:0,1"), Some(UseStep::Menu(0, 1)));
        assert_eq!(UseStep::parse("pick:2"), Some(UseStep::Pick(2)));
        assert_eq!(UseStep::parse("pull:-1,0"), Some(UseStep::Pull(-1, 0)));
        assert_eq!(UseStep::parse("nope"), None);
    }
}
