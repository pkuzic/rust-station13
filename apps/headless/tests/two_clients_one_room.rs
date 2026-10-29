//! Engine-foundation exit criterion: two clients connect over UDP to one server, move
//! with server authority, and see each other at the same positions.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use rs13_classic::maps;
use rs13_headless::{BotConfig, BotReport, SeenPlayer, bot_app, unique_client_id};
use rs13_world::{Dir, TileMap};

fn run_bot(server: SocketAddr, dir: Dir) -> thread::JoinHandle<Vec<SeenPlayer>> {
    thread::spawn(move || {
        let report = BotReport::default();
        let config = BotConfig {
            server,
            client_id: unique_client_id(),
            dir: Some(dir),
            // Long enough to cross the room and stop at a wall.
            hold: Duration::from_secs(5),
        };
        let exit = bot_app(config, report.clone()).run();
        assert!(exit.is_success(), "bot moving {dir:?} exited with {exit:?}");
        report.lock().unwrap().take().expect("bot produced a report")
    })
}

#[test]
fn two_clients_move_in_one_room() {
    // A port derived from the process id so parallel test runs do not collide.
    let port = 20_000 + (std::process::id() % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    // The server runs until the test process exits.
    thread::spawn(move || rs13_server::server_app(port).run());
    thread::sleep(Duration::from_millis(500));

    let east = run_bot(server, Dir::East);
    thread::sleep(Duration::from_millis(300));
    let north = run_bot(server, Dir::North);
    let east = east.join().unwrap();
    let north = north.join().unwrap();

    let map = TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).unwrap();
    for (report, dir) in [(&east, Dir::East), (&north, Dir::North)] {
        assert_eq!(report.len(), 2, "each bot sees both players: {report:?}");
        let own = report
            .iter()
            .find(|p| p.own)
            .expect("bot knows which player it controls");
        assert!(
            map.is_solid(own.pos.step(dir)),
            "bot moving {dir:?} should have stopped against a wall, stopped at {:?}",
            own.pos
        );
    }

    // Both clients agree on where everyone is (both were standing still at the end).
    let positions = |r: &[SeenPlayer]| {
        let mut v: Vec<_> = r.iter().map(|p| (p.client_id, p.pos)).collect();
        v.sort();
        v
    };
    assert_eq!(positions(&east), positions(&north));
}
