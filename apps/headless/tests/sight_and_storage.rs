//! What the server sends follows what a mob can see (`docs/reference/lighting.md`): a
//! person behind your back isn't sent until you turn round, and what is inside a first aid
//! kit isn't sent until you open it. Driven end to end, over UDP, by scripted bots.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use rs13_headless::{BotConfig, BotReport, BotResult, Snapshot, Step, bot_app, unique_client_id};
use rs13_network::protocol::ActionInput;
use rs13_world::Dir;

/// Spawn points fill from the south: the first bot to join stands at `1`, the second at
/// `2`, three tiles north of it, with a first aid kit beside it.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        '1': (tile: (id: "floor"), spawn: true),
        '2': (tile: (id: "floor"), spawn: true),
        'k': (tile: (id: "floor"), objects: ["first-aid-kit"]),
    },
    rows: [
        "WWWWWWW",
        "W.....W",
        "W..2k.W",
        "W.....W",
        "W.....W",
        "W..1..W",
        "WWWWWWW",
    ],
)"#;

const KIT: [&str; 3] = ["bruise-pack", "ointment", "health-analyzer"];

fn run_bot(server: SocketAddr, script: Vec<Step>, hold: Duration) -> thread::JoinHandle<BotResult> {
    thread::spawn(move || {
        let report = BotReport::default();
        let config = BotConfig {
            server,
            client_id: unique_client_id(),
            dir: None,
            hold,
            examine: None,
            prefer_job: None,
            name: None,
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

/// Other people sent: the map has no lights, so they come as silhouettes (`dark_people.rs`).
fn others(s: &Snapshot) -> usize {
    s.players.iter().filter(|p| !p.own).count() + s.silhouettes.len()
}

fn has(s: &Snapshot, sprite: &str) -> bool {
    s.sprites.iter().any(|x| x == sprite)
}

#[test]
fn people_behind_you_and_what_is_in_a_kit_are_sent_only_once_seen() {
    let port = 20_000 + ((std::process::id() + 7_919) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_map(port, MAP).run());
    thread::sleep(Duration::from_millis(500));

    // The first bot stands still, facing south, south of where the second will be.
    let south = run_bot(
        server,
        vec![wait(4_500), Step::Snapshot("end".into())],
        Duration::from_secs(6),
    );
    thread::sleep(Duration::from_millis(500));
    // The second turns its back on the first, then round again (Ctrl + a direction in the
    // client), then picks the kit up and opens it.
    let viewer = run_bot(
        server,
        vec![
            wait(300),
            Step::Act(ActionInput::Face(Dir::North)),
            wait(500),
            Step::Snapshot("start".into()),
            Step::Act(ActionInput::Face(Dir::South)),
            wait(500),
            Step::Snapshot("turned".into()),
            Step::UseOn("first-aid-kit".into()),
            wait(400),
            Step::Snapshot("holding".into()),
            // An empty hand on the kit in the other hand opens it.
            Step::Act(ActionInput::SwapHands),
            wait(300),
            Step::UseOn("first-aid-kit".into()),
            wait(700),
            Step::Snapshot("opened".into()),
        ],
        Duration::from_secs(5),
    );
    let south = south.join().unwrap();
    let viewer = viewer.join().unwrap();

    let start = snapshot(&viewer, "start");
    assert_eq!(
        others(start),
        0,
        "the person behind our back isn't sent: {start:?}"
    );
    assert!(has(start, "first-aid-kit"), "{start:?}");
    assert!(
        !KIT.iter().any(|k| has(start, k)),
        "the kit's contents aren't sent while shut: {start:?}"
    );

    let turned = snapshot(&viewer, "turned");
    assert_eq!(others(turned), 1, "turned round, they are: {turned:?}");

    let holding = snapshot(&viewer, "holding");
    assert!(has(holding, "first-aid-kit"), "{holding:?}");
    assert!(
        !KIT.iter().any(|k| has(holding, k)),
        "held but still shut: {holding:?}"
    );

    let opened = snapshot(&viewer, "opened");
    for item in KIT {
        assert!(has(opened, item), "{item} sent once open: {opened:?}");
    }
    let storage = viewer.storage.as_ref().expect("the kit's window was sent");
    assert_eq!(storage.items.len(), 5, "{storage:?}");

    // The first bot, facing south, had the second behind its back the whole time.
    let end = snapshot(&south, "end");
    assert_eq!(others(end), 0, "{end:?}");
    assert_eq!(south.under_floor_received, 0);
}

/// A closet east of the only spawn point, against a wall.
const CLOSET_MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        '1': (tile: (id: "floor"), spawn: true),
        'c': (tile: (id: "floor"), objects: ["closet"]),
    },
    rows: [
        "WWWWW",
        "W1cWW",
        "WWWWW",
    ],
)"#;

#[test]
fn shut_in_a_closet_you_still_have_yourself() {
    let port = 20_000 + ((std::process::id() + 13_331) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_map(port, CLOSET_MAP).run());
    thread::sleep(Duration::from_millis(500));

    // Open the closet, step in, and shut it from inside.
    let bot = run_bot(
        server,
        vec![
            wait(300),
            Step::UseOn("closet".into()),
            wait(400),
            Step::Move(Some(Dir::East)),
            wait(500),
            Step::Move(None),
            Step::Snapshot("in the open closet".into()),
            Step::UseOn("closet".into()),
            wait(600),
            Step::Snapshot("shut in".into()),
        ],
        Duration::from_secs(4),
    )
    .join()
    .unwrap();

    let open = snapshot(&bot, "in the open closet");
    let me = open.players.iter().find(|p| p.own).expect("own player");
    assert_eq!((me.pos.x, me.pos.y), (2, 1), "stepped into the closet: {open:?}");
    let shut = snapshot(&bot, "shut in");
    assert!(
        shut.players.iter().any(|p| p.own),
        "own player still sent: {shut:?}"
    );
    assert!(has(shut, "closet"), "and the closet around us: {shut:?}");
}

/// A row of airlocks with fire shutters along the south of a room, seen from far along it.
const EDGE_MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        '1': (tile: (id: "floor"), spawn: true),
        'D': (tile: (id: "floor"), objects: ["airlock", "firedoor"]),
        'f': (tile: (id: "floor"), objects: ["firedoor"]),
    },
    rows: [
        "WWWWWWWW",
        "W1.....W",
        "W......W",
        "WDDDDDDW",
        "W...f..W",
        "WWWWWWWW",
    ],
)"#;

/// What's on a closed door's tile arrives whenever the door shows, even with its centre out of
/// sight behind the doors next to it, and what's beyond the doors doesn't.
#[test]
fn what_shares_a_shown_doors_tile_is_sent_with_it() {
    let port = 20_000 + ((std::process::id() + 15_101) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_map(port, EDGE_MAP).run());
    thread::sleep(Duration::from_millis(500));
    let viewer = run_bot(
        server,
        vec![wait(1_500), Step::Snapshot("end".into())],
        Duration::from_secs(3),
    );
    let result = viewer.join().unwrap();
    let seen = snapshot(&result, "end");
    let on = |sprite: &str, x: i32, y: i32| {
        seen.placed
            .iter()
            .any(|(s, p)| s == sprite && p.x == x && p.y == y)
    };
    // Rows count from the south: the doors are row 2, the shutter beyond them row 1.
    for x in 1..=6 {
        assert!(on("airlock", x, 2), "airlock at {x}");
        assert!(
            on("firedoor", x, 2),
            "the shutter on the airlock at {x}: {:?}",
            seen.placed
        );
    }
    assert!(!on("firedoor", 4, 1), "the shutter beyond them");
}
