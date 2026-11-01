//! Someone standing in the dark is sent to others as a silhouette only
//! (`docs/reference/lighting.md`, Silhouettes): where they are, not who they are, what they
//! wear or hold. Stepping into the light, they are sent in full. Driven end to end, over
//! UDP, by scripted bots.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use rs13_headless::{BotConfig, BotReport, BotResult, Snapshot, Step, bot_app, unique_client_id};
use rs13_network::protocol::ActionInput;
use rs13_world::{Dir, Intent, TilePos};

/// A corridor with a tube at its west end. Spawn points fill from the west: the first bot
/// stands by the tube, in the light; the second seven tiles from it, in the dark, but in
/// sight of the first.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'l': (tile: (id: "floor"), objects: [(id: "light", dir: West)]),
        '1': (tile: (id: "floor"), spawn: true),
        '2': (tile: (id: "floor"), spawn: true),
    },
    rows: [
        "WWWWWWWWWWWWW",
        "Wl1.....2...W",
        "WWWWWWWWWWWWW",
    ],
)"#;

const STRANGER: &str = "Dark Stranger";

fn run_bot(
    server: SocketAddr,
    name: &str,
    script: Vec<Step>,
    hold: Duration,
) -> thread::JoinHandle<BotResult> {
    let name = name.to_owned();
    thread::spawn(move || {
        let report = BotReport::default();
        let config = BotConfig {
            server,
            client_id: unique_client_id(),
            dir: None,
            hold,
            examine: None,
            name: Some(name),
            say: None,
            script,
        };
        let exit = bot_app(config, report.clone()).run();
        assert!(exit.is_success(), "bot exited with {exit:?}");
        report.lock().unwrap().take().expect("bot produced a report")
    })
}

fn snapshot<'a>(result: &'a BotResult, label: &str) -> &'a Snapshot {
    result
        .snapshots
        .iter()
        .find(|s| s.label == label)
        .unwrap_or_else(|| panic!("no snapshot {label}: {:?}", result.snapshots))
}

fn wait(ms: u64) -> Step {
    Step::Wait(Duration::from_millis(ms))
}

fn others(s: &Snapshot) -> usize {
    s.players.iter().filter(|p| !p.own).count()
}

#[test]
fn someone_in_the_dark_is_sent_as_a_shape_until_they_step_into_the_light() {
    let port = 20_000 + ((std::process::id() + 4_441) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_map(port, MAP).run());
    thread::sleep(Duration::from_millis(500));

    // The first bot looks east down the corridor and notes what it is sent, before and
    // after the second walks into the light.
    let viewer = run_bot(
        server,
        "Viewer",
        vec![
            Step::Act(ActionInput::Face(Dir::East)),
            wait(1_800),
            Step::Snapshot("in the dark".into()),
            wait(4_700),
            Step::Snapshot("in the light".into()),
        ],
        Duration::from_secs(8),
    );
    thread::sleep(Duration::from_millis(400));
    // The second waits in the dark, well after the first has looked (a busy machine can
    // run late), then walks west, towards the tube.
    let stranger = run_bot(
        server,
        STRANGER,
        vec![
            wait(3_500),
            Step::Move(Some(Dir::West)),
            wait(900),
            Step::Move(None),
            wait(300),
            Step::Snapshot("walked".into()),
        ],
        Duration::from_secs(7),
    );
    let viewer = viewer.join().unwrap();
    let stranger = stranger.join().unwrap();

    let dark = snapshot(&viewer, "in the dark");
    assert_eq!(others(dark), 0, "the person isn't sent: {dark:?}");
    assert_eq!(
        dark.silhouettes,
        vec![TilePos::new(8, 1, 0)],
        "only their shape is, where they stand: {dark:?}"
    );
    assert!(
        !dark.names.iter().any(|n| n == STRANGER),
        "not their name: {:?}",
        dark.names
    );
    // Our own mob and what we wear and hold, plus their shape: nothing of theirs.
    let mine = snapshot(&stranger, "walked").sprites.len();
    assert!(
        dark.sprites.len() < mine + 2,
        "none of what they wear: {:?}",
        dark.sprites
    );

    let lit = snapshot(&viewer, "in the light");
    let stranger_pos = snapshot(&stranger, "walked")
        .players
        .iter()
        .find(|p| p.own)
        .expect("own player")
        .pos;
    assert!(stranger_pos.x < 7, "walked towards the tube: {stranger_pos:?}");
    assert_eq!(others(lit), 1, "now they are sent: {lit:?}");
    assert!(lit.silhouettes.is_empty(), "and the shape is gone: {lit:?}");
    assert!(lit.names.iter().any(|n| n == STRANGER), "{:?}", lit.names);
}

/// No lights: two people side by side, both in the dark.
const UNLIT: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        '1': (tile: (id: "floor"), spawn: true),
        '2': (tile: (id: "floor"), spawn: true),
    },
    rows: [
        "WWWWWW",
        "W.12.W",
        "WWWWWW",
    ],
)"#;

#[test]
fn hitting_a_shape_in_the_dark_hits_the_person() {
    let port = 20_000 + ((std::process::id() + 6_007) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_map(port, UNLIT).run());
    thread::sleep(Duration::from_millis(500));

    // The first bot, set to harm, punches the only other "human" it is sent: the
    // second bot's silhouette.
    let attacker = run_bot(
        server,
        "Attacker",
        vec![
            Step::Act(ActionInput::Face(Dir::East)),
            Step::Act(ActionInput::SetIntent(Intent::Harm)),
            wait(1_200),
            Step::Snapshot("before".into()),
            // Five punches: each misses one time in ten.
            Step::UseOn("human".into()),
            wait(250),
            Step::UseOn("human".into()),
            wait(250),
            Step::UseOn("human".into()),
            wait(250),
            Step::UseOn("human".into()),
            wait(250),
            Step::UseOn("human".into()),
        ],
        Duration::from_secs(4),
    );
    thread::sleep(Duration::from_millis(400));
    let victim = run_bot(
        server,
        STRANGER,
        vec![wait(2_800), Step::Snapshot("after".into())],
        Duration::from_secs(4),
    );
    let attacker = attacker.join().unwrap();
    let victim = victim.join().unwrap();

    let before = snapshot(&attacker, "before");
    assert_eq!(before.silhouettes, vec![TilePos::new(3, 1, 0)], "{before:?}");
    assert_eq!(others(before), 0, "{before:?}");
    let health = snapshot(&victim, "after").status.as_ref().expect("status").health;
    assert!(health < 100, "the punches landed on the person: {health}");
}
