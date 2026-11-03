//! Talking out of character, with bots over UDP against a real server. Spec: docs/reference/speech.md.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use rs13_headless::{BotConfig, BotReport, BotResult, Step, bot_app, unique_client_id};
use rs13_i18n::LocArg;
use rs13_network::protocol::ActionInput;

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        's': (tile: (id: "floor"), spawn: true),
    },
    rows: ["WWWWWWWWWWWWWWWWWWWWWWW", "Ws...................sW", "WWWWWWWWWWWWWWWWWWWWWWW"],
)"#;

fn bot(server: SocketAddr, name: &str, script: Vec<Step>) -> thread::JoinHandle<BotResult> {
    let name = name.to_owned();
    thread::spawn(move || {
        let report = BotReport::default();
        let config = BotConfig {
            server,
            client_id: unique_client_id(),
            dir: None,
            hold: Duration::from_millis(1_500),
            examine: None,
            prefer_job: None,
            name: Some(name),
            say: None,
            script,
        };
        let exit = bot_app(config, report.clone()).run();
        assert!(exit.is_success(), "bot exited with {exit:?}");
        report.lock().unwrap().take().expect("bot produced a report")
    })
}

/// OOC reaches the far side of the station, where speech wouldn't.
#[test]
fn ooc_reaches_everyone_however_far() {
    let port = 20_000 + ((std::process::id() + 26_006) % 20_000) as u16;
    let server: SocketAddr = ([127, 0, 0, 1], port).into();
    thread::spawn(move || rs13_server::server_app_with_map(port, MAP).run());
    thread::sleep(Duration::from_millis(500));
    let wait = Step::Wait(Duration::from_millis(800));
    let speaker = bot(
        server,
        "Speaker",
        vec![
            wait.clone(),
            Step::Act(ActionInput::Ooc("hello everyone".to_owned())),
            Step::Act(ActionInput::Say("not you".to_owned())),
        ],
    );
    let listener = bot(server, "Listener", vec![wait.clone(), wait]);
    let (speaker, listener) = (speaker.join().unwrap(), listener.join().unwrap());
    let says = |result: &BotResult, id: &str| {
        result.notices.iter().any(|m| {
            m.id == id
                && m.args
                    .iter()
                    .any(|(k, v)| k == "name" && *v == LocArg::Str("Speaker".to_owned()))
        })
    };
    assert!(says(&speaker, "ooc-say"), "{:?}", speaker.notices);
    assert!(says(&listener, "ooc-say"), "{:?}", listener.notices);
    assert!(says(&speaker, "say"), "{:?}", speaker.notices);
    assert!(!says(&listener, "say"), "out of earshot: {:?}", listener.notices);
}
