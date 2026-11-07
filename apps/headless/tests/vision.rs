//! X-ray and blindness over the network (`docs/reference/genetics.md`): with X-ray, one bot sees a
//! monkey shut behind two walls that the other doesn't; then it's injected with blindness and sees
//! nothing but itself. An injector's X-ray only takes at its chance, so the server gives it here.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use bevy::prelude::*;
use rs13_classic::movement::Mob;
use rs13_headless::{BotConfig, BotReport, BotResult, Snapshot, Step, bot_app, unique_client_id};
use rs13_world::{DisplayName, Vision};

/// Two start points side by side, the blinding injector beside both, and a monkey in a room of
/// its own.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        's': (tile: (id: "floor"), spawn: true),
        'b': (tile: (id: "floor"), objects: ["dna-injector-blind"]),
        'm': (tile: (id: "floor"), objects: ["monkey"]),
        'L': (tile: (id: "floor"), objects: [(id: "light", dir: North)]),
    },
    rows: [
        "WWWWWWWWW",
        "WssLWW.mW",
        "W.b.WW..W",
        "WWWWWWWWW",
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

/// Ann has X-ray from the start, until her genes say otherwise.
fn give_x_ray(mut commands: Commands, mobs: Query<(Entity, &DisplayName), (With<Mob>, Without<Vision>)>) {
    for (mob, name) in &mobs {
        if name.0 == "Ann Subject" {
            commands.entity(mob).insert(Vision::Xray);
        }
    }
}

#[test]
fn x_ray_sees_through_walls_and_the_blind_see_nothing() {
    let port = 20_000 + ((std::process::id() + 13_771) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || {
        let mut app = rs13_server::server_app_with_map(port, MAP);
        app.add_systems(Update, give_x_ray);
        app.run()
    });
    thread::sleep(Duration::from_millis(500));

    let wait = |ms| Step::Wait(Duration::from_millis(ms));
    let subject = vec![
        wait(2_000),
        Step::Snapshot("x-ray".into()),
        wait(7_000),
        Step::Snapshot("blind".into()),
    ];
    let geneticist = vec![
        wait(2_000),
        Step::Snapshot("plain".into()),
        Step::UseOn("dna-injector-blind".into()),
        wait(400),
        Step::UseOn("human".into()),
        // Injecting takes four seconds.
        wait(6_000),
    ];
    let subject = thread::spawn(move || bot(server, "Ann Subject", 11, subject));
    thread::sleep(Duration::from_millis(300));
    let geneticist = bot(server, "Gene Tist", 10, geneticist);
    let subject = subject.join().unwrap();

    let snap = |result: &BotResult, label: &str| -> Snapshot {
        result
            .snapshots
            .iter()
            .find(|s| s.label == label)
            .unwrap_or_else(|| panic!("no snapshot {label}: {:?}", result.snapshots))
            .clone()
    };
    let vision = |s: &Snapshot| s.status.as_ref().map(|s| s.vision);
    let sees = |s: &Snapshot, sprite: &str| s.sprites.iter().any(|x| x == sprite);
    let others = |s: &Snapshot| s.players.iter().filter(|p| !p.own).count();

    let plain = snap(&geneticist, "plain");
    assert_eq!(vision(&plain), Some(Vision::Normal), "{plain:?}");
    assert!(!sees(&plain, "monkey"), "walls hide the monkey: {plain:?}");
    assert_eq!(others(&plain), 1, "Ann beside them: {plain:?}");

    let xray = snap(&subject, "x-ray");
    assert_eq!(vision(&xray), Some(Vision::Xray), "{xray:?}");
    assert!(sees(&xray, "monkey"), "through the walls: {xray:?}");
    assert_eq!(others(&xray), 1, "{xray:?}");

    let blind = snap(&subject, "blind");
    assert_eq!(
        vision(&blind),
        Some(Vision::Blind),
        "{blind:?}\n{:?}",
        geneticist.notices
    );
    assert!(
        !sees(&blind, "monkey") && !sees(&blind, "light") && others(&blind) == 0,
        "only herself: {blind:?}"
    );
}
