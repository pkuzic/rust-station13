//! Dedicated server: engine plus the classic game build.
//!
//! Usage: `rs13-server [--port N] [--map engineering-chamber|luna-engineering|luna-toxins|luna-incinerator|luna-station] [--map-file PATH]...`
//!
//! `--map-file` loads a map in the RON map format from disk, for trying things out. Given
//! more than once, each file is the next deck up of one station.

use bevy::log::LogPlugin;
use bevy::prelude::*;
use rs13_classic::maps;

fn main() -> AppExit {
    let mut port = rs13_network::DEFAULT_PORT;
    let mut decks: Vec<&'static str> = Vec::new();
    // The whole station runs rounds; a map for trying things out starts you straight in it.
    let mut rounds: Option<bool> = None;
    let mut pregame = rs13_classic::round::PREGAME_SECONDS;
    let mut pod_seconds: Option<u32> = None;
    let mut forced_mode: Option<&'static str> = None;
    let mut vote_seconds: Option<u32> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--rounds" => {
                rounds = Some(true);
                continue;
            }
            "--no-rounds" => {
                rounds = Some(false);
                continue;
            }
            _ => {}
        }
        match (arg.as_str(), args.next()) {
            ("--mode", Some(value)) => {
                let built = rs13_classic::round::MODES
                    .iter()
                    .find(|m| m.built && m.id == value);
                match built {
                    Some(mode) => forced_mode = Some(mode.id),
                    None => return usage(),
                }
            }
            ("--vote-seconds", Some(value)) => match value.parse() {
                Ok(v) => vote_seconds = Some(v),
                Err(_) => return usage(),
            },
            ("--pod-seconds", Some(value)) => match value.parse() {
                Ok(p) => pod_seconds = Some(p),
                Err(_) => return usage(),
            },
            ("--pregame", Some(value)) => match value.parse() {
                Ok(p) => pregame = p,
                Err(_) => return usage(),
            },
            ("--port", Some(value)) => match value.parse() {
                Ok(p) => port = p,
                Err(_) => return usage(),
            },
            ("--map", Some(name)) => match name.as_str() {
                "engineering-chamber" => decks = vec![maps::ENGINEERING_CHAMBER],
                "luna-engineering" => decks = vec![maps::LUNA_ENGINEERING],
                "luna-toxins" => decks = vec![maps::LUNA_TOXINS],
                "luna-incinerator" => decks = vec![maps::LUNA_INCINERATOR],
                "luna-station" => decks = maps::LUNA_STATION.to_vec(),
                _ => return usage(),
            },
            ("--map-file", Some(path)) => match std::fs::read_to_string(&path) {
                // Read once and kept for the life of the server.
                Ok(text) => decks.push(Box::leak(text.into_boxed_str())),
                Err(e) => {
                    eprintln!("rs13-server: can't read {path}: {e}");
                    return AppExit::error();
                }
            },
            _ => return usage(),
        }
    }
    if decks.is_empty() {
        decks.push(maps::ENGINEERING_CHAMBER);
    }
    let station = decks == maps::LUNA_STATION.to_vec();
    let rounds = rounds.unwrap_or(station);
    let mut logging = true;
    // When a round is over, the server starts again with a clean station.
    loop {
        let mut app = if rounds {
            rs13_server::server_app_with_rounds(port, decks.clone(), pregame)
        } else {
            rs13_server::server_app_with_station(port, decks.clone())
        };
        // What the players last voted for outlasts a restart.
        if let Ok(voted) = rs13_server::vote::VOTED_MODE.lock()
            && let Some(mode) = *voted
        {
            forced_mode = Some(mode).filter(|m| *m != rs13_server::vote::SECRET);
        }
        if let Some(seconds) = vote_seconds {
            app.insert_resource(rs13_server::vote::Votes {
                period_ds: seconds as f32 * 10.0,
                ..Default::default()
            });
        }
        if let Some(mode) = forced_mode
            && rounds
        {
            app.world_mut()
                .resource_mut::<rs13_classic::round::Round>()
                .forced_mode = Some(mode);
        }
        if let Some(seconds) = pod_seconds {
            let mut pods = rs13_classic::escape::EscapePods::default();
            pods.launch_ds = seconds as f32 * 10.0;
            pods.transit_ds = seconds as f32 * 10.0;
            app.insert_resource(pods);
        }
        if logging {
            app.add_plugins(LogPlugin::default());
            logging = false;
        }
        let exit = app.run();
        if exit != AppExit::from_code(rs13_server::RESTART_CODE) {
            return exit;
        }
    }
}

fn usage() -> AppExit {
    eprintln!(
        "usage: rs13-server [--port N] [--map engineering-chamber|luna-engineering|luna-toxins|luna-incinerator|luna-station] [--map-file PATH]..."
    );
    AppExit::error()
}
