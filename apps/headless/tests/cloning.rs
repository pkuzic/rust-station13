//! Cloning over the network (`docs/reference/cloning.md`): a player scanned while alive is burnt
//! in a crematorium and comes back as a clone, still the same client.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use rs13_headless::{BotConfig, BotReport, BotResult, Snapshot, Step, bot_app, unique_client_id};
use rs13_world::MobStat;

/// The first to join stands below the second, who reaches the scanner, console, crematorium and
/// its tray, its igniter and a card. The pod is one further along.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        '1': (tile: (id: "floor"), spawn: true),
        '2': (tile: (id: "floor"), spawn: true),
        'c': (tile: (id: "floor"), objects: [(id: "crematorium", vars: {"id": "crema"})]),
        's': (tile: (id: "floor"), objects: [(id: "crematorium-igniter", vars: {"id": "crema"})]),
        'i': (tile: (id: "floor"), objects: [(id: "id-card", access: ["crematorium"])]),
        'n': (tile: (id: "floor"), objects: ["dna-scanner"]),
        'k': (tile: (id: "floor"), objects: ["cloning-console"]),
        'p': (tile: (id: "floor"), objects: ["clone-pod"]),
    },
    rows: [
        "WWWWWWWWWW",
        "W..cknp..W",
        "W...2s...W",
        "W...1i...W",
        "WWWWWWWWWW",
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
        name: Some(name.to_owned()),
        say: None,
        script,
    };
    assert!(bot_app(config, report.clone()).run().is_success());
    report.lock().unwrap().take().unwrap()
}

#[test]
fn a_burnt_player_who_was_scanned_comes_back_as_a_clone() {
    let port = 20_000 + ((std::process::id() + 9_211) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_map(port, MAP).run());
    thread::sleep(Duration::from_millis(500));

    let wait = |ms| Step::Wait(Duration::from_millis(ms));
    let operator = vec![
        wait(1_200),
        // Scan them.
        Step::Pull("human".into()),
        wait(400),
        Step::UseOn("dna-scanner".into()),
        wait(400),
        Step::UseOn("cloning-console".into()),
        wait(400),
        Step::UseOn("dna-scanner".into()),
        wait(400),
        // Burn them.
        Step::UseOn("crematorium".into()),
        wait(300),
        Step::Pull("human".into()),
        wait(400),
        Step::UseOn("crematorium-tray".into()),
        wait(400),
        Step::UseOn("crematorium".into()),
        wait(400),
        Step::UseOn("id-card".into()),
        wait(400),
        Step::UseOn("crematorium-igniter".into()),
        wait(14_000),
        Step::UseOn("cloning-console".into()),
        wait(3_000),
    ];
    let victim = vec![
        wait(19_000),
        Step::Snapshot("a ghost".into()),
        wait(5_500),
        Step::Snapshot("in the pod".into()),
    ];
    let victim = thread::spawn(move || bot(server, "Ann Hart", 28, victim));
    thread::sleep(Duration::from_millis(500));
    let operator = bot(server, "Medic", 26, operator);
    let victim = victim.join().unwrap();

    let ids = |r: &BotResult| r.notices.iter().map(|m| m.id.clone()).collect::<Vec<_>>();
    let snap = |label: &str| -> Snapshot {
        victim
            .snapshots
            .iter()
            .find(|s| s.label == label)
            .unwrap()
            .clone()
    };
    assert!(
        ids(&operator).contains(&"cloning-scanned".to_owned()),
        "{:?}",
        operator.notices
    );
    assert!(
        ids(&operator).contains(&"cloning-started".to_owned()),
        "{:?}",
        operator.notices
    );
    assert_eq!(snap("a ghost").status.unwrap().stat, MobStat::Ghost);
    let pod = snap("in the pod");
    assert_ne!(pod.status.as_ref().unwrap().stat, MobStat::Ghost, "{pod:?}");
    assert_eq!(pod.players.iter().filter(|p| p.own).count(), 1, "{pod:?}");
    assert!(
        ids(&victim).contains(&"cloning-begins".to_owned()),
        "{:?}",
        victim.notices
    );
}
