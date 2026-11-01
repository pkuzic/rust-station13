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
