//! Headless bot client as a library, so tests can run bots in-process.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use rs13_i18n::LocMsg;
use rs13_network::client::{ClientNetworkPlugin, connect};
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, MoveInput, Notice, Player, StatusUpdate};
use rs13_world::{Appearance, Dir, GridPos, NetId, TilePos};

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
    /// Examine the first object with this sprite id once it is visible.
    pub examine: Option<String>,
    /// Character name to ask for after connecting.
    pub name: Option<String>,
    /// Something to say one second before finishing.
    pub say: Option<String>,
}

/// A player as the bot last saw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SeenPlayer {
    pub client_id: u64,
    pub pos: TilePos,
    pub own: bool,
}

/// What the bot saw by the time it finished.
#[derive(Debug, Clone, Default)]
pub struct BotResult {
    pub players: Vec<SeenPlayer>,
    /// Notices from the server, unformatted, in arrival order.
    pub notices: Vec<LocMsg>,
    /// The last status the server reported for the bot's own mob.
    pub status: Option<StatusUpdate>,
}

/// Filled in when the bot finishes. `None` if it never received its own player.
pub type BotReport = Arc<Mutex<Option<BotResult>>>;

/// Notices received so far, and the latest status.
#[derive(Resource, Default)]
struct Received(Vec<LocMsg>, Option<StatusUpdate>);

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
    .init_resource::<Received>()
    .add_systems(Startup, start)
    .add_systems(
        Update,
        (send_input, examine, talk, receive_notices, finish).chain(),
    );
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

/// Ask once, after connecting, for the configured name and movement. The server keeps
/// applying the movement.
fn send_input(
    config: Res<BotConfig>,
    mut senders: Query<
        (&mut MessageSender<MoveInput>, &mut MessageSender<ActionInput>),
        (With<Client>, With<Connected>),
    >,
    mut sent: Local<bool>,
) {
    if *sent {
        return;
    }
    let Ok((mut moves, mut actions)) = senders.single_mut() else {
        return;
    };
    if let Some(name) = &config.name {
        actions.send::<IntentChannel>(ActionInput::SetName(name.clone()));
    }
    moves.send::<IntentChannel>(MoveInput {
        dir: config.dir,
        run: true,
    });
    *sent = true;
}

/// Say the configured line once, a second before the bot finishes.
fn talk(
    time: Res<Time>,
    config: Res<BotConfig>,
    joined: Res<JoinedAt>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
    mut done: Local<bool>,
) {
    let (Some(line), Some(at)) = (&config.say, joined.0) else {
        return;
    };
    if *done || time.elapsed() + Duration::from_secs(1) < at + config.hold {
        return;
    }
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    sender.send::<IntentChannel>(ActionInput::Say(line.clone()));
    *done = true;
}

/// Examine the configured object once, as soon as it has been replicated.
fn examine(
    config: Res<BotConfig>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
    objects: Query<(&NetId, &Appearance)>,
    mut done: Local<bool>,
) {
    let Some(sprite) = &config.examine else { return };
    if *done {
        return;
    }
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    if let Some((id, _)) = objects.iter().find(|(_, a)| &a.sprite == sprite) {
        sender.send::<IntentChannel>(ActionInput::Examine(*id));
        *done = true;
    }
}

/// Receivers are queried separately: Lightyear adds each one to the client entity only
/// once the first message of that type arrives.
fn receive_notices(
    mut notices: Query<&mut MessageReceiver<Notice>, With<Client>>,
    mut statuses: Query<&mut MessageReceiver<StatusUpdate>, With<Client>>,
    mut received: ResMut<Received>,
) {
    for mut receiver in &mut notices {
        received.0.extend(receiver.receive().map(|n| n.0));
    }
    for mut receiver in &mut statuses {
        if let Some(status) = receiver.receive().last() {
            received.1 = Some(status);
        }
    }
}

fn finish(
    time: Res<Time>,
    config: Res<BotConfig>,
    sink: Res<ReportSink>,
    received: Res<Received>,
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
            *sink.0.lock().unwrap() = Some(BotResult {
                players: seen,
                notices: received.0.clone(),
                status: received.1,
            });
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}
