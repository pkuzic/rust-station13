//! Dedicated server: engine plus the classic game build.
//!
//! Usage: `rs13-server [--port N] [--map engineering-chamber|luna-engineering] [--map-file PATH]`
//!
//! `--map-file` loads a map in the RON map format from disk, for trying things out.

use bevy::log::LogPlugin;
use bevy::prelude::*;
use rs13_classic::maps;

fn main() -> AppExit {
    let mut port = rs13_network::DEFAULT_PORT;
    let mut map = maps::ENGINEERING_CHAMBER;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match (arg.as_str(), args.next()) {
            ("--port", Some(value)) => match value.parse() {
                Ok(p) => port = p,
                Err(_) => return usage(),
            },
            ("--map", Some(name)) => match name.as_str() {
                "engineering-chamber" => map = maps::ENGINEERING_CHAMBER,
                "luna-engineering" => map = maps::LUNA_ENGINEERING,
                _ => return usage(),
            },
            ("--map-file", Some(path)) => match std::fs::read_to_string(&path) {
                // Read once and kept for the life of the server.
                Ok(text) => map = Box::leak(text.into_boxed_str()),
                Err(e) => {
                    eprintln!("rs13-server: can't read {path}: {e}");
                    return AppExit::error();
                }
            },
            _ => return usage(),
        }
    }
    let mut app = rs13_server::server_app_with_map(port, map);
    app.add_plugins(LogPlugin::default());
    app.run()
}

fn usage() -> AppExit {
    eprintln!("usage: rs13-server [--port N] [--map engineering-chamber|luna-engineering] [--map-file PATH]");
    AppExit::error()
}
