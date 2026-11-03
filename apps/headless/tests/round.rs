//! The lobby and the start of a round, over UDP. Spec: docs/reference/round.md.

use std::net::SocketAddr;
use std::thread;
use std::time::{Duration, Instant};

use rs13_headless::{BotConfig, BotReport, BotResult, Step, bot_app, unique_client_id};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'e': (tile: (id: "floor"), spawn: true, objects: [(id: "id-card", vars: {"job": "engineer"})]),
        'm': (tile: (id: "floor"), spawn: true, objects: [(id: "id-card", vars: {"job": "medical-doctor"})]),
        'k': (tile: (id: "floor"), objects: ["crowbar"]),
    },
    rows: [
        "WWWWWWW",
        "We.k.mW",
        "WWWWWWW",
    ],
)"#;

fn run_bot(server: SocketAddr, prefer_job: Option<u8>) -> thread::JoinHandle<(BotResult, Duration)> {
    thread::spawn(move || {
        let started = Instant::now();
        let report = BotReport::default();
        let config = BotConfig {
            server,
            client_id: unique_client_id(),
            dir: None,
            hold: Duration::from_millis(1_500),
            examine: None,
            prefer_job,
            name: None,
            say: None,
            script: vec![
                Step::Wait(Duration::from_millis(500)),
                Step::Snapshot("in".into()),
            ],
        };
        let exit = bot_app(config, report.clone()).run();
        assert!(exit.is_success(), "bot exited with {exit:?}");
        let result = report.lock().unwrap().take().expect("bot produced a report");
        (result, started.elapsed())
    })
}

/// Bots ask to join as they connect, so the first waits for the round, and the second, later on, is
/// put straight in. The first prefers a doctor, though the list would make it an engineer.
#[test]
fn people_wait_for_the_round_and_join_after_it() {
    let port = 20_000 + ((std::process::id() + 23_003) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_rounds(port, vec![MAP], 5).run());
    thread::sleep(Duration::from_millis(500));
    let line = rs13_classic::jobs::jobs()
        .iter()
        .position(|j| j.id == "medical-doctor")
        .unwrap() as u8;

    let first = run_bot(server, Some(line));
    thread::sleep(Duration::from_secs(7));
    let second = run_bot(server, None);
    let (first, waited) = first.join().unwrap();
    let (second, quick) = second.join().unwrap();

    assert!(
        waited >= Duration::from_millis(4_500),
        "waited for the round: {waited:?}"
    );
    assert!(quick < Duration::from_secs(5), "no wait once it's on: {quick:?}");
    let spot = |result: &BotResult| {
        let snapshot = result
            .snapshots
            .iter()
            .find(|s| s.label == "in")
            .expect("in the world");
        let own = snapshot.players.iter().find(|p| p.own).expect("its own mob");
        (own.pos.x, own.pos.y)
    };
    assert_eq!(spot(&first), (5, 1), "the doctor's post, as asked");
    assert_eq!(spot(&second), (1, 1), "the post that's left");
}
