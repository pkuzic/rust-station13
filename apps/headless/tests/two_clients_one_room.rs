//! Engine-foundation exit criterion: two clients connect over UDP to one server, move
//! with server authority, and see each other at the same positions. Also round-trips
//! examine, character names and speech through the network.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use rs13_classic::{locales, maps};
use rs13_headless::{BotConfig, BotReport, BotResult, SeenPlayer, bot_app, unique_client_id};
use rs13_i18n::{Lexicon, Localizer, langid};
use rs13_world::{Dir, TileMap};

#[derive(Default)]
struct Extra {
    examine: Option<&'static str>,
    name: Option<&'static str>,
    say: Option<&'static str>,
}

fn run_bot(server: SocketAddr, dir: Dir, extra: Extra) -> thread::JoinHandle<BotResult> {
    thread::spawn(move || {
        let report = BotReport::default();
        let config = BotConfig {
            server,
            client_id: unique_client_id(),
            dir: Some(dir),
            // Long enough to cross the room and stop.
            hold: Duration::from_secs(5),
            examine: extra.examine.map(str::to_owned),
            name: extra.name.map(str::to_owned),
            say: extra.say.map(str::to_owned),
        };
        let exit = bot_app(config, report.clone()).run();
        assert!(exit.is_success(), "bot moving {dir:?} exited with {exit:?}");
        report.lock().unwrap().take().expect("bot produced a report")
    })
}

fn localizers() -> (Localizer, Localizer) {
    let en = Localizer::new(
        langid!("en"),
        &[locales::EN_MESSAGES],
        Lexicon::from_ron(locales::EN_NOUNS).unwrap(),
    )
    .unwrap();
    let ru = Localizer::new(
        langid!("ru"),
        &[locales::RU_MESSAGES],
        Lexicon::from_ron(locales::RU_NOUNS).unwrap(),
    )
    .unwrap();
    (en, ru)
}

#[test]
fn two_clients_move_in_one_room() {
    // A port derived from the process id so parallel test runs do not collide.
    let port = 20_000 + (std::process::id() % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    // The server runs until the test process exits.
    thread::spawn(move || rs13_server::server_app(port).run());
    thread::sleep(Duration::from_millis(500));

    // Both walk east along the same row: the second follows the first and stops next
    // to it, so they are in earshot when the second one speaks near the end.
    let first = run_bot(
        server,
        Dir::East,
        Extra {
            examine: Some("crowbar"),
            ..Extra::default()
        },
    );
    thread::sleep(Duration::from_millis(300));
    let second = run_bot(
        server,
        Dir::East,
        Extra {
            name: Some("bob   barnes"),
            say: Some("hello there"),
            ..Extra::default()
        },
    );
    let first = first.join().unwrap();
    let second = second.join().unwrap();

    let map = TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).unwrap();
    for result in [&first, &second] {
        let report = &result.players;
        assert_eq!(report.len(), 2, "each bot sees both players: {report:?}");
        let own = report
            .iter()
            .find(|p| p.own)
            .expect("bot knows which player it controls");
        let ahead = own.pos.step(Dir::East);
        assert!(
            map.is_solid(ahead) || report.iter().any(|p| p.pos == ahead),
            "bot should have stopped against a wall or the other player, stopped at {:?}",
            own.pos
        );
    }

    // Both clients agree on where everyone is (both were standing still at the end).
    let positions = |r: &[SeenPlayer]| {
        let mut v: Vec<_> = r.iter().map(|p| (p.client_id, p.pos)).collect();
        v.sort();
        v
    };
    assert_eq!(positions(&first.players), positions(&second.players));

    let (en, ru) = localizers();
    let ids = |r: &BotResult| r.notices.iter().map(|n| n.id.clone()).collect::<Vec<_>>();

    // The examine answer went only to the bot that asked.
    assert_eq!(ids(&first), ["examine", "say"]);
    assert_eq!(en.format_msg(&first.notices[0]), "That's a crowbar.");
    assert_eq!(ru.format_msg(&first.notices[0]), "Это лом.");

    // The second bot's name was cleaned up by the server, and both heard its words.
    assert_eq!(ids(&second), ["say"]);
    for result in [&first, &second] {
        let said = result.notices.iter().find(|n| n.id == "say").unwrap();
        assert_eq!(en.format_msg(said), "Bob Barnes says, \"Hello there\"");
        assert_eq!(ru.format_msg(said), "Bob Barnes говорит: «Hello there»");
    }
}
