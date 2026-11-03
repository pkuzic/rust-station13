//! Votes, with bots over UDP against a real server. Spec: docs/reference/round.md.

use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use bevy::app::AppExit;
use rs13_headless::{BotConfig, BotReport, Step, bot_app, unique_client_id};
use rs13_network::protocol::{ActionInput, VoteAction};
use rs13_server::RESTART_CODE;
use rs13_server::vote::Votes;

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        's': (tile: (id: "floor"), spawn: true),
    },
    rows: ["WWWWW", "Ws.sW", "WWWWW"],
)"#;

fn run_bot(server: SocketAddr, script: Vec<Step>, hold_ms: u64) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let config = BotConfig {
            server,
            client_id: unique_client_id(),
            dir: None,
            hold: Duration::from_millis(hold_ms),
            examine: None,
            prefer_job: None,
            name: None,
            say: None,
            script,
        };
        let _ = bot_app(config, BotReport::default()).run();
    })
}

fn server(offset: u32, votes_ds: f32) -> (SocketAddr, thread::JoinHandle<AppExit>) {
    let port = 20_000 + ((std::process::id() + offset) % 20_000) as u16;
    let handle = thread::spawn(move || {
        let mut app = rs13_server::server_app_with_map(port, MAP);
        app.insert_resource(Votes {
            period_ds: votes_ds,
            ..Votes::default()
        });
        app.run()
    });
    thread::sleep(Duration::from_millis(500));
    (([127, 0, 0, 1], port).into(), handle)
}

fn act(action: VoteAction) -> Step {
    Step::Act(ActionInput::Vote(action))
}

/// One yes and nobody against restarts the server.
#[test]
fn a_restart_vote_that_passes_restarts_the_server() {
    let (address, server) = server(25_005, 20.0);
    let bot = run_bot(
        address,
        vec![
            Step::Wait(Duration::from_millis(500)),
            act(VoteAction::BeginRestart),
            Step::Wait(Duration::from_millis(300)),
            act(VoteAction::Cast(1)),
        ],
        6_000,
    );
    let exit = server.join().unwrap();
    assert_eq!(exit, AppExit::from_code(RESTART_CODE));
    let _ = bot.join();
}
