//! A station of two decks: a player walks up the stairs and is sent the upper deck, then
//! walks back down. Driven end to end, over UDP, by a scripted bot. Spec:
//! docs/reference/multiz.md.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use rs13_headless::{BotConfig, BotReport, BotResult, Snapshot, Step, bot_app, unique_client_id};
use rs13_world::{Dir, TilePos};

/// The lower deck: the bot starts on the bottom stairs' entry (4, 1); their active tile is
/// north of it. A chair stands in the room.
const LOWER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'h': (tile: (id: "floor"), objects: ["chair"]),
        'e': (tile: (id: "floor"), spawn: true, objects: [(id: "stairs-entry-bottom", dir: North)]),
        'a': (tile: (id: "floor"), objects: [(id: "stairs-up", dir: North)]),
    },
    rows: [
        "WWWWWW",
        "W.h.aW",
        "W...eW",
        "WWWWWW",
    ],
)"#;

/// The upper deck: the top stairs mirrored over the bottom ones, and a closet.
const UPPER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'c': (tile: (id: "floor"), objects: ["closet"]),
        'e': (tile: (id: "floor"), objects: [(id: "stairs-entry", dir: North)]),
        'a': (tile: (id: "floor"), objects: [(id: "stairs-down", dir: North)]),
    },
    rows: [
        "WWWWWW",
        "Wc..eW",
        "W...aW",
        "WWWWWW",
    ],
)"#;

fn snapshot<'a>(result: &'a BotResult, label: &str) -> &'a Snapshot {
    result
        .snapshots
        .iter()
        .find(|s| s.label == label)
        .unwrap_or_else(|| panic!("no snapshot {label}: {:?}", result.snapshots))
}

fn own(s: &Snapshot) -> TilePos {
    s.players.iter().find(|p| p.own).expect("own player").pos
}

fn wait(ms: u64) -> Step {
    Step::Wait(Duration::from_millis(ms))
}

#[test]
fn a_player_walks_up_the_stairs_and_sees_the_upper_deck() {
    let port = 20_000 + ((std::process::id() + 8_111) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_station(port, vec![LOWER, UPPER]).run());
    thread::sleep(Duration::from_millis(500));

    let bot = thread::spawn(move || {
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
            script: vec![
                wait(300),
                Step::Snapshot("below".into()),
                Step::Move(Some(Dir::North)),
                wait(300),
                Step::Move(None),
                wait(500),
                Step::Snapshot("above".into()),
                Step::Move(Some(Dir::South)),
                wait(300),
                Step::Move(None),
                wait(500),
                Step::Snapshot("back".into()),
            ],
        };
        let exit = bot_app(config, report.clone()).run();
        assert!(exit.is_success(), "bot exited with {exit:?}");
        report.lock().unwrap().take().expect("bot produced a report")
    })
    .join()
    .unwrap();

    let below = snapshot(&bot, "below");
    assert_eq!(own(below), TilePos::new(4, 1, 0));
    assert!(below.sprites.iter().any(|s| s == "chair"), "{below:?}");
    assert!(
        !below.sprites.iter().any(|s| s == "closet"),
        "the upper deck isn't sent"
    );

    let above = snapshot(&bot, "above");
    assert_eq!(
        own(above),
        TilePos::new(4, 2, 1),
        "up the stairs, onto the top entry"
    );
    assert!(
        above.placed.iter().any(|(s, p)| s == "closet" && p.z == 1),
        "the upper deck is sent: {above:?}"
    );
    assert!(
        !above.placed.iter().any(|(s, p)| s == "chair" && p.z == 0),
        "and the lower deck no longer: {above:?}"
    );

    assert_eq!(
        own(snapshot(&bot, "back")),
        TilePos::new(4, 1, 0),
        "and down again"
    );
}

/// A hole upstairs over a wrench, with someone standing on it. A crowbar away from the hole isn't sent.
const HOLE_LOWER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'b': (tile: (id: "floor"), spawn: true, objects: ["wrench"]),
        'c': (tile: (id: "floor"), objects: ["crowbar"]),
        'l': (tile: (id: "floor"), objects: [(id: "light", dir: West)]),
    },
    rows: [
        "WWWWWWWW",
        "Wl.b.c.W",
        "WWWWWWWW",
    ],
)"#;

const HOLE_UPPER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'a': (tile: (id: "floor"), spawn: true, objects: ["screwdriver"]),
        'o': (tile: (id: "open-space")),
    },
    rows: [
        "WWWWWWWW",
        "W.ao...W",
        "WWWWWWWW",
    ],
)"#;

/// The one above sees who and what is under the hole, the one below sees nothing upstairs, and they
/// hear each other.
#[test]
fn what_is_under_a_hole_is_seen_and_heard_from_above() {
    let port = 20_000 + ((std::process::id() + 24_411) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_station(port, vec![HOLE_LOWER, HOLE_UPPER]).run());
    thread::sleep(Duration::from_millis(500));
    let bot = |line: &'static str| {
        thread::spawn(move || {
            let report = BotReport::default();
            let config = BotConfig {
                server,
                client_id: unique_client_id(),
                dir: None,
                hold: Duration::from_millis(4_000),
                examine: None,
                prefer_job: None,
                name: None,
                say: Some(line.to_owned()),
                script: vec![
                    Step::Wait(Duration::from_millis(1_500)),
                    Step::Snapshot("seen".into()),
                ],
            };
            let exit = bot_app(config, report.clone()).run();
            assert!(exit.is_success(), "bot exited with {exit:?}");
            report.lock().unwrap().take().expect("bot produced a report")
        })
    };
    let (one, two) = (bot("Hello from one"), bot("Hello from two"));
    let results: Vec<BotResult> = vec![one.join().unwrap(), two.join().unwrap()];
    let shot = |r: &BotResult| r.snapshots.iter().find(|s| s.label == "seen").unwrap().clone();
    let own_deck = |r: &BotResult| shot(r).players.iter().find(|p| p.own).unwrap().pos.z;
    let above = results.iter().find(|r| own_deck(r) == 1).expect("one upstairs");
    let below = results.iter().find(|r| own_deck(r) == 0).expect("one downstairs");

    let has = |list: &[(String, TilePos)], what: &str| list.iter().any(|(s, _)| s == what);
    let seen = shot(above);
    assert_eq!(seen.players.len(), 2, "the bot below: {:?}", seen.players);
    assert!(has(&seen.placed, "wrench"), "{:?}", seen.placed);
    assert!(!has(&seen.placed, "crowbar"), "{:?}", seen.placed);

    let seen = shot(below);
    assert_eq!(seen.players.len(), 1, "can't see up: {:?}", seen.players);
    assert!(!has(&seen.placed, "screwdriver"), "{:?}", seen.placed);

    let said = |r: &BotResult| format!("{:?}", r.notices);
    assert!(said(above).contains("Hello from"), "{}", said(above));
    assert!(said(below).contains("Hello from"), "{}", said(below));
}
