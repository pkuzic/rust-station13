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
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match (arg.as_str(), args.next()) {
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
    let mut app = rs13_server::server_app_with_station(port, decks);
    app.add_plugins(LogPlugin::default());
    app.run()
}

fn usage() -> AppExit {
    eprintln!(
        "usage: rs13-server [--port N] [--map engineering-chamber|luna-engineering|luna-toxins|luna-incinerator|luna-station] [--map-file PATH]..."
    );
    AppExit::error()
}
