//! Headless bot client for automated scenarios and smoke tests.
//!
//! Connects to a server, holds one movement direction for a while, then prints every
//! player it can see and exits.
//!
//! Usage: `rs13-headless [--server ADDR] [--dir north|east|...] [--seconds N]`
//!
//! Output (stdout, one line per player): `player <client_id> <x> <y> [self]`.
//! Exit code 1 if the bot never received its own player.

use std::net::SocketAddr;
use std::time::Duration;

use bevy::log::LogPlugin;
use bevy::prelude::*;
use rs13_headless::{BotConfig, BotReport, bot_app, unique_client_id};
use rs13_world::Dir;

fn main() -> AppExit {
    let config = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rs13-headless: {e}");
            return AppExit::from_code(2);
        }
    };
    let report = BotReport::default();
    let mut app = bot_app(config, report.clone());
    app.add_plugins(LogPlugin::default());
    let exit = app.run();

    for player in report.lock().unwrap().iter().flatten() {
        let tag = if player.own { " self" } else { "" };
        println!(
            "player {} {} {}{tag}",
            player.client_id, player.pos.x, player.pos.y
        );
    }
    exit
}

fn parse_args() -> Result<BotConfig, String> {
    let mut config = BotConfig {
        server: SocketAddr::from(([127, 0, 0, 1], rs13_network::DEFAULT_PORT)),
        client_id: unique_client_id(),
        dir: None,
        hold: Duration::from_secs(3),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().ok_or_else(|| format!("{arg} needs a value"))?;
        match arg.as_str() {
            "--server" => config.server = value.parse().map_err(|e| format!("--server: {e}"))?,
            "--dir" => config.dir = Some(parse_dir(&value)?),
            "--seconds" => {
                config.hold = Duration::from_secs_f32(value.parse().map_err(|e| format!("--seconds: {e}"))?)
            }
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    Ok(config)
}

fn parse_dir(s: &str) -> Result<Dir, String> {
    Ok(match s {
        "north" => Dir::North,
        "northeast" => Dir::NorthEast,
        "east" => Dir::East,
        "southeast" => Dir::SouthEast,
        "south" => Dir::South,
        "southwest" => Dir::SouthWest,
        "west" => Dir::West,
        "northwest" => Dir::NorthWest,
        _ => return Err(format!("unknown direction {s}")),
    })
}
