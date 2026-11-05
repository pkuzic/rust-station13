//! Admins, with bots over UDP against a real server. Spec: docs/reference/admin.md.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use rs13_headless::{BotConfig, BotReport, BotResult, Step, bot_app, unique_client_id};
use rs13_network::protocol::ActionInput;
use rs13_server::admin::{AdminConfig, key_of, level_of, parse_admins, parse_bans};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        's': (tile: (id: "floor"), spawn: true),
    },
    rows: ["WWWWWWWWWWWWWWWWWWWW", "Wssssss............W", "WWWWWWWWWWWWWWWWWWWW"],
)"#;

fn server(offset: u32, admins: &[(&str, &str)]) -> SocketAddr {
    let port = 20_000 + ((std::process::id() + offset) % 20_000) as u16;
    let admins: HashMap<String, i32> = admins
        .iter()
        .map(|(secret, rank)| (key_of(secret), level_of(rank).unwrap()))
        .collect();
    thread::spawn(move || {
        let mut app = rs13_server::server_app_with_map(port, MAP);
        app.insert_resource(AdminConfig {
            admins,
            bans: Vec::new(),
            bans_file: None,
            ooc_on: true,
        });
        app.run()
    });
    thread::sleep(Duration::from_millis(500));
    ([127, 0, 0, 1], port).into()
}

fn bot(
    server: SocketAddr,
    secret: &str,
    name: &str,
    hold: u64,
    then: Vec<Step>,
) -> thread::JoinHandle<BotResult> {
    let mut script = vec![Step::Act(ActionInput::Hello(secret.to_owned()))];
    script.extend(then);
    let name = name.to_owned();
    thread::spawn(move || {
        let report = BotReport::default();
        let config = BotConfig {
            server,
            client_id: unique_client_id(),
            dir: None,
            hold: Duration::from_millis(hold),
            examine: None,
            prefer_job: None,
            name: Some(name),
            say: None,
            script,
        };
        let _ = bot_app(config, report.clone()).run();
        report.lock().unwrap().take().expect("bot produced a report")
    })
}

fn wait(ms: u64) -> Step {
    Step::Wait(Duration::from_millis(ms))
}

fn admin(line: &str) -> Step {
    Step::Act(ActionInput::Admin(line.to_owned()))
}

fn heard(result: &BotResult, id: &str) -> bool {
    result.notices.iter().any(|m| m.id == id)
}

#[test]
fn ranks_and_files_read_as_lunas_do() {
    let admins = parse_admins("; a comment\nabc - Host\ndef - Moderator\nghi - Nonsense\n");
    assert_eq!(admins.get("abc"), Some(&6));
    assert_eq!(admins.get("def"), Some(&0));
    assert_eq!(admins.len(), 2);
    let bans = parse_bans("abc|127.0.0.1|123|Me|griefing\ndef|||Me|for good\n");
    assert_eq!(bans[0].until, Some(123));
    assert_eq!(bans[1].until, None);
    assert_eq!(bans[1].address, None);
}

#[test]
fn a_keys_the_same_each_time_and_only_for_that_secret() {
    assert_eq!(key_of("one"), key_of("one"));
    assert_ne!(key_of("one"), key_of("two"));
    assert_eq!(key_of("one").len(), 16);
}

#[test]
fn only_admins_get_verbs_and_rank_counts() {
    let server = server(28_008, &[("the-host", "Host"), ("the-mod", "Moderator")]);
    let host = bot(
        server,
        "the-host",
        "Hosty",
        6_000,
        vec![wait(600), admin("who"), wait(1_500)],
    );
    let moderator = bot(
        server,
        "the-mod",
        "Moddy",
        6_000,
        vec![wait(600), admin("spawn crowbar"), wait(1_500)],
    );
    let player = bot(
        server,
        "a-player",
        "Plain",
        6_000,
        vec![
            wait(600),
            admin("who"),
            admin("key"),
            Step::Act(ActionInput::AdminHelp("help me".to_owned())),
            wait(1_500),
        ],
    );
    let (host, moderator, player) = (
        host.join().unwrap(),
        moderator.join().unwrap(),
        player.join().unwrap(),
    );
    assert!(heard(&host, "admin-welcome"), "{:?}", host.notices);
    assert!(heard(&host, "admin-who-row"), "{:?}", host.notices);
    assert!(
        heard(&host, "admin-help-from"),
        "admins see adminhelp: {:?}",
        host.notices
    );
    assert!(heard(&moderator, "admin-low-rank"), "{:?}", moderator.notices);
    assert!(heard(&player, "admin-not-admin"), "{:?}", player.notices);
    assert!(heard(&player, "admin-your-key"), "{:?}", player.notices);
    assert!(!heard(&player, "admin-help-from"), "{:?}", player.notices);
}

#[test]
fn a_ban_drops_them_and_keeps_them_out() {
    let server = server(29_009, &[("the-host", "Host")]);
    let victim = bot(server, "bad-player", "Baddie", 5_000, vec![wait(5_000)]);
    let host = bot(
        server,
        "the-host",
        "Hosty",
        5_000,
        vec![
            wait(1_500),
            admin("ban baddie 60 griefing"),
            wait(1_000),
            admin("bans"),
            wait(1_000),
        ],
    );
    let (victim, host) = (victim.join().unwrap(), host.join().unwrap());
    assert!(heard(&victim, "admin-banned"), "{:?}", victim.notices);
    assert!(heard(&host, "admin-ban-row"), "{:?}", host.notices);

    let back = bot(server, "bad-player", "Baddie", 2_000, vec![wait(1_500)]);
    assert!(heard(&back.join().unwrap(), "admin-banned"), "still out");
}

#[test]
fn muted_people_cant_speak_or_use_ooc() {
    let server = server(30_010, &[("the-host", "Host")]);
    let victim = bot(
        server,
        "loud",
        "Loud",
        7_000,
        vec![
            wait(2_000),
            Step::Act(ActionInput::Ooc("still here".to_owned())),
            Step::Act(ActionInput::Say("hello".to_owned())),
            wait(1_500),
        ],
    );
    let host = bot(
        server,
        "the-host",
        "Hosty",
        7_000,
        vec![wait(800), admin("mute loud"), wait(4_000)],
    );
    let (victim, host) = (victim.join().unwrap(), host.join().unwrap());
    assert!(heard(&victim, "admin-muted"), "{:?}", victim.notices);
    assert!(!heard(&victim, "say"), "{:?}", victim.notices);
    assert!(!heard(&host, "ooc-say"), "{:?}", host.notices);
}
