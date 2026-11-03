//! Pulling over the network (`docs/reference/pulling.md`): Ctrl-click a crate, walk, and it
//! follows; the HUD pull button lights up, and lets go when clicked.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use rs13_headless::{BotConfig, BotReport, Snapshot, Step, bot_app, unique_client_id};
use rs13_network::protocol::ActionInput;
use rs13_world::{Dir, TilePos};

/// Rows run top down: the bot starts at (3, 2) with a crate east of it.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        '1': (tile: (id: "floor"), spawn: true),
        'c': (tile: (id: "floor"), objects: ["crate"]),
    },
    rows: [
        "WWWWWWW",
        "W..1c.W",
        "W.....W",
        "WWWWWWW",
    ],
)"#;

fn crate_at(s: &Snapshot) -> TilePos {
    s.placed
        .iter()
        .find(|(sprite, _)| sprite == "crate")
        .map(|(_, p)| *p)
        .unwrap_or_else(|| panic!("no crate: {s:?}"))
}

fn pulling(s: &Snapshot) -> bool {
    s.status.as_ref().is_some_and(|st| st.pulling)
}

#[test]
fn a_crate_pulled_over_the_network_follows_and_lets_go() {
    let port = 20_000 + ((std::process::id() + 4_441) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_map(port, MAP).run());
    thread::sleep(Duration::from_millis(500));

    let wait = |ms| Step::Wait(Duration::from_millis(ms));
    let script = vec![
        wait(300),
        Step::Snapshot("before".into()),
        Step::Pull("crate".into()),
        wait(300),
        Step::Snapshot("pulling".into()),
        // West until the wall: two steps, the crate following.
        Step::Move(Some(Dir::West)),
        wait(900),
        Step::Move(None),
        Step::Snapshot("walked".into()),
        // The HUD pull button lets go; stepping south (diagonal to it) leaves it behind.
        Step::Act(ActionInput::StopPulling),
        wait(300),
        Step::Move(Some(Dir::South)),
        wait(900),
        Step::Move(None),
        Step::Snapshot("let go".into()),
    ];
    let report = BotReport::default();
    let config = BotConfig {
        server,
        client_id: unique_client_id(),
        dir: None,
        hold: Duration::from_secs(5),
        examine: None,
        prefer_job: None,
        name: None,
        say: None,
        script,
    };
    assert!(bot_app(config, report.clone()).run().is_success());
    let result = report.lock().unwrap().take().unwrap();
    let snap = |label: &str| {
        result
            .snapshots
            .iter()
            .find(|s| s.label == label)
            .unwrap_or_else(|| panic!("no {label}: {:?}", result.snapshots))
            .clone()
    };
    let own = |s: &Snapshot| s.players.iter().find(|p| p.own).unwrap().pos;

    let before = snap("before");
    assert_eq!(crate_at(&before), TilePos::new(4, 2, 0));
    assert!(!pulling(&before));

    let started = snap("pulling");
    assert!(pulling(&started), "the pull button lights up: {started:?}");

    let walked = snap("walked");
    assert_eq!(own(&walked), TilePos::new(1, 2, 0), "{walked:?}");
    assert_eq!(
        crate_at(&walked),
        TilePos::new(2, 2, 0),
        "the crate followed: {walked:?}"
    );
    assert!(pulling(&walked));

    let let_go = snap("let go");
    assert!(!pulling(&let_go), "{let_go:?}");
    assert_eq!(own(&let_go), TilePos::new(1, 1, 0), "{let_go:?}");
    assert_eq!(
        crate_at(&let_go),
        TilePos::new(2, 2, 0),
        "left behind: {let_go:?}"
    );
}
