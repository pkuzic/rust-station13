//! Slipping, with a bot over UDP against a real server. Spec: docs/reference/combat.md.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use rs13_headless::{BotConfig, BotReport, Step, bot_app, unique_client_id};
use rs13_world::Dir;

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        's': (tile: (id: "floor"), spawn: true),
        'o': (tile: (id: "floor"), objects: ["soap"]),
    },
    rows: ["WWWWWWW", "Wso...W", "WWWWWWW"],
)"#;

#[test]
fn walking_onto_soap_slips_you_up() {
    let port = 20_000 + ((std::process::id() + 27_007) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_map(port, MAP).run());
    thread::sleep(Duration::from_millis(500));
    let report = BotReport::default();
    let config = BotConfig {
        server,
        client_id: unique_client_id(),
        dir: None,
        hold: Duration::from_millis(4_000),
        examine: None,
        prefer_job: None,
        name: None,
        say: None,
        script: vec![
            Step::Wait(Duration::from_millis(500)),
            Step::Move(Some(Dir::East)),
            Step::Wait(Duration::from_millis(1_500)),
            Step::Move(None),
            Step::Snapshot("after".into()),
        ],
    };
    let exit = bot_app(config, report.clone()).run();
    assert!(exit.is_success(), "bot exited with {exit:?}");
    let result = report.lock().unwrap().take().expect("bot produced a report");
    assert!(
        result.notices.iter().any(|m| m.id == "slipped"),
        "{:?}",
        result.notices
    );
    let own = result.snapshots[0]
        .players
        .iter()
        .find(|p| p.own)
        .expect("its own mob");
    assert_eq!(own.pos.x, 2, "on the soap, and no further");
}
