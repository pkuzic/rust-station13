//! Ghosts over the network (`docs/reference/ghosts.md`): one bot burns another in a
//! crematorium. The dead one walks on through walls and can't be seen or heard by the living.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use rs13_headless::{BotConfig, BotReport, BotResult, Step, bot_app, unique_client_id};
use rs13_network::protocol::ActionInput;
use rs13_world::{Dir, MobStat, TilePos};

/// The first to join stands below the second, who reaches the crematorium, its tray, its
/// igniter and a card.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        '1': (tile: (id: "floor"), spawn: true),
        '2': (tile: (id: "floor"), spawn: true),
        'c': (tile: (id: "floor"), objects: [(id: "crematorium", vars: {"id": "crema"})]),
        's': (tile: (id: "floor"), objects: [(id: "crematorium-igniter", vars: {"id": "crema"})]),
        'i': (tile: (id: "floor"), objects: [(id: "id-card", access: ["crematorium"])]),
    },
    rows: [
        "WWWWWWWW",
        "W.WcsiWW",
        "W.W.2..W",
        "W.W.1..W",
        "WWWWWWWW",
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
fn the_burnt_player_walks_on_unseen_by_the_living() {
    let port = 20_000 + ((std::process::id() + 7_123) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_map(port, MAP).run());
    thread::sleep(Duration::from_millis(500));

    let wait = |ms| Step::Wait(Duration::from_millis(ms));
    let stoker = vec![
        wait(1_500),
        Step::UseOn("crematorium".into()),
        wait(2_500),
        // Pull the other one onto the tray, which goes back with them on it.
        Step::Pull("human".into()),
        wait(400),
        Step::UseOn("crematorium-tray".into()),
        wait(400),
        Step::UseOn("crematorium".into()),
        wait(400),
        Step::UseOn("id-card".into()),
        wait(400),
        Step::UseOn("crematorium-igniter".into()),
        wait(16_000),
        Step::Act(ActionInput::Say("Rest in peace".into())),
        wait(2_500),
        Step::Snapshot("after".into()),
    ];

    let victim = vec![
        wait(2_500),
        wait(15_000),
        Step::Snapshot("dead".into()),
        // Through the wall to the west.
        Step::Move(Some(Dir::West)),
        wait(350),
        Step::Move(None),
        wait(300),
        Step::Snapshot("through the wall".into()),
        Step::Act(ActionInput::Say("Boo".into())),
        wait(1_000),
        Step::Act(ActionInput::Respawn),
        wait(800),
        Step::Snapshot("new life".into()),
    ];
    let victim = thread::spawn(move || bot(server, "Ann Hart", 23, victim));
    thread::sleep(Duration::from_millis(500));
    let stoker = bot(server, "Stoker", 26, stoker);
    let victim = victim.join().unwrap();

    let snap = |result: &BotResult, label: &str| {
        result
            .snapshots
            .iter()
            .find(|s| s.label == label)
            .unwrap_or_else(|| panic!("no snapshot {label}: {:?}", result.snapshots))
            .clone()
    };
    let own = |s: &rs13_headless::Snapshot| -> TilePos {
        s.players.iter().find(|p| p.own).expect("our own player").pos
    };

    let dead = snap(&victim, "dead");
    assert_eq!(dead.status.as_ref().unwrap().stat, MobStat::Ghost, "{dead:?}");
    assert!(dead.status.as_ref().unwrap().alerts.is_empty());
    assert!(
        dead.names.contains(&"Stoker".to_owned()),
        "ghosts see the living: {dead:?}"
    );
    assert_eq!(own(&dead), TilePos::new(3, 3, 0), "it leaves from where it burnt");

    let walked = own(&snap(&victim, "through the wall"));
    assert!(walked.x < 2, "through the wall at x = 2: {walked:?}");

    let ids = |result: &BotResult| -> Vec<String> { result.notices.iter().map(|m| m.id.clone()).collect() };
    assert!(
        ids(&victim).contains(&"ghost-left-body".to_owned()),
        "{:?}",
        victim.notices
    );
    assert!(
        ids(&victim).contains(&"ghost-say".to_owned()),
        "{:?}",
        victim.notices
    );
    assert!(
        ids(&victim).contains(&"say".to_owned()),
        "ghosts listen: {:?}",
        victim.notices
    );
    assert!(
        !ids(&stoker).contains(&"ghost-say".to_owned()),
        "{:?}",
        stoker.notices
    );

    let new_life = snap(&victim, "new life");
    assert_eq!(
        new_life.status.as_ref().unwrap().stat,
        MobStat::Conscious,
        "{new_life:?}"
    );
    assert_eq!(
        own(&new_life),
        TilePos::new(4, 1, 0),
        "at the free start point: {new_life:?}"
    );
    assert_eq!(new_life.players.iter().filter(|p| p.own).count(), 1);

    let after = snap(&stoker, "after");
    assert!(
        !after.names.contains(&"Ann Hart".to_owned()),
        "the living are never sent a ghost: {after:?}"
    );
}
