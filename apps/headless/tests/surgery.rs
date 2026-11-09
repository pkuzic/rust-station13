//! Surgery over the network (`docs/reference/surgery.md`): a surgeon lays someone on the
//! operating table with a grab, drapes their groin and makes the first incision.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use rs13_headless::{BotConfig, BotReport, BotResult, Step, bot_app, unique_client_id};
use rs13_network::protocol::ActionInput;
use rs13_world::Intent;

/// The patient joins first, below the surgeon, who reaches the table, the drapes and a scalpel.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        '1': (tile: (id: "floor"), spawn: true),
        '2': (tile: (id: "floor"), spawn: true),
        'T': (tile: (id: "floor"), objects: ["operating-table"]),
        'd': (tile: (id: "floor"), objects: ["surgical-drapes"]),
        'k': (tile: (id: "floor"), objects: ["scalpel"]),
    },
    rows: [
        "WWWWWWW",
        "W.dTk.W",
        "W..2..W",
        "W..1..W",
        "WWWWWWW",
    ],
)"#;

fn bot(server: SocketAddr, name: &str, hold: u64, script: Vec<Step>) -> BotResult {
    let report = BotReport::default();
    let config = BotConfig {
        server,
        client_id: unique_client_id(),
        dir: None,
        hold: Duration::from_secs(hold),
        examine: None,
        prefer_job: None,
        name: Some(name.to_owned()),
        say: None,
        script,
    };
    assert!(bot_app(config, report.clone()).run().is_success());
    report.lock().unwrap().take().unwrap()
}

#[test]
fn a_patient_laid_on_the_table_is_draped_and_cut() {
    let port = 20_000 + ((std::process::id() + 4_733) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_map(port, MAP).run());
    thread::sleep(Duration::from_millis(500));

    let wait = |ms| Step::Wait(Duration::from_millis(ms));
    let surgeon = vec![
        wait(1_500),
        // Grab them and lay them on the table.
        Step::Act(ActionInput::SetIntent(Intent::Grab)),
        wait(200),
        Step::UseOn("human".into()),
        wait(400),
        Step::UseOn("operating-table".into()),
        wait(400),
        Step::Act(ActionInput::SetIntent(Intent::Help)),
        // Drapes over the groin: the appendectomy is the first on the list.
        Step::Act(ActionInput::SetZone(2)),
        Step::UseOn("surgical-drapes".into()),
        wait(400),
        Step::UseOn("human".into()),
        wait(400),
        Step::PickOn("human".into(), 0),
        wait(400),
        // Put the drapes down and cut.
        Step::Act(ActionInput::Drop),
        wait(300),
        Step::UseOn("scalpel".into()),
        wait(400),
        Step::UseOn("human".into()),
        wait(3_500),
    ];
    let patient = thread::spawn(move || bot(server, "Pat Ient", 16, vec![wait(14_000)]));
    thread::sleep(Duration::from_millis(500));
    let surgeon = bot(server, "Doc Tor", 14, surgeon);
    let _ = patient.join().unwrap();

    let ids: Vec<String> = surgeon.notices.iter().map(|m| m.id.clone()).collect();
    for id in [
        "surgery-laid-on-table",
        "surgery-draped",
        "surgery-begin-incise",
        "surgery-done-incise",
    ] {
        assert!(ids.contains(&id.to_owned()), "no {id} in {ids:?}");
    }
}
