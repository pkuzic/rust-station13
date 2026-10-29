//! Headless bot client as a library, so tests can run bots in-process.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use rs13_network::client::{ClientNetworkPlugin, connect};
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{IntentChannel, MoveInput, Player};
use rs13_world::{Dir, GridPos, TilePos};

/// Give up if no own player has been received after this long.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Resource, Clone, Debug)]
pub struct BotConfig {
    pub server: SocketAddr,
    pub client_id: u64,
    /// Direction to hold for the whole run; `None` stands still.
    pub dir: Option<Dir>,
    /// How long to keep running after the bot's own player appears.
    pub hold: Duration,
}

/// A player as the bot last saw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SeenPlayer {
    pub client_id: u64,
    pub pos: TilePos,
    pub own: bool,
}

/// What the bot saw when it finished. `None` if it never received its own player.
pub type BotReport = Arc<Mutex<Option<Vec<SeenPlayer>>>>;

#[derive(Resource)]
struct ReportSink(BotReport);

/// When the bot first saw its own player, if it has.
#[derive(Resource, Default)]
struct JoinedAt(Option<Duration>);

/// A bot ready to `run()`. When it exits, `report` holds what it saw.
/// Logging is left to the caller.
pub fn bot_app(config: BotConfig, report: BotReport) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(5))),
        StatesPlugin,
    ))
    .add_plugins(ClientNetworkPlugin)
    .insert_resource(config)
    .insert_resource(ReportSink(report))
    .init_resource::<JoinedAt>()
    .add_systems(Startup, start)
    .add_systems(Update, (send_input, finish).chain());
    app
}

/// Distinct per bot without pulling in a random number generator.
pub fn unique_client_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    nanos ^ (u64::from(std::process::id()) << 32) ^ n.rotate_right(8)
}

fn start(mut commands: Commands, config: Res<BotConfig>) -> Result {
    info!(server = %config.server, client_id = config.client_id, "connecting");
    connect(&mut commands, config.server, config.client_id)?;
    Ok(())
}

/// Ask once, after connecting, for the configured movement. The server keeps applying it.
fn send_input(
    config: Res<BotConfig>,
    mut sender: Query<&mut MessageSender<MoveInput>, (With<Client>, With<Connected>)>,
    mut sent: Local<bool>,
) {
    if *sent {
        return;
    }
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    sender.send::<IntentChannel>(MoveInput {
        dir: config.dir,
        run: true,
    });
    *sent = true;
}

fn finish(
    time: Res<Time>,
    config: Res<BotConfig>,
    sink: Res<ReportSink>,
    mut joined: ResMut<JoinedAt>,
    players: Query<(&Player, &GridPos, Has<Controlled>)>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed();
    if joined.0.is_none() && players.iter().any(|(.., own)| own) {
        joined.0 = Some(now);
        info!("own player received");
    }
    match joined.0 {
        None if now > CONNECT_TIMEOUT => {
            warn!("no own player received within {CONNECT_TIMEOUT:?}");
            exit.write(AppExit::from_code(1));
        }
        Some(at) if now > at + config.hold => {
            let mut seen: Vec<SeenPlayer> = players
                .iter()
                .map(|(player, pos, own)| SeenPlayer {
                    client_id: player.client_id,
                    pos: pos.0,
                    own,
                })
                .collect();
            seen.sort();
            *sink.0.lock().unwrap() = Some(seen);
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}
