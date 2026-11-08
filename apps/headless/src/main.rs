//! Headless bot client for automated scenarios and smoke tests.
//!
//! Connects to a server, holds one movement direction for a while, then prints every
//! player it can see and exits.
//!
//! Usage: `rs13-headless [--server ADDR] [--dir north|east|...] [--seconds N] [--examine SPRITE]
//! [--name NAME] [--say TEXT] [--use SPRITE]... [--pull SPRITE]... [--cast LINE:SPRITE]... [--equip SLOT]... [--use-held] [--wait SECONDS]...`
//!
//! `--use` and `--wait` make a script, run in order: `--use` clicks the first thing with
//! that sprite on the map (picking it up), `--pull` starts pulling one, `--use-held` clicks the item in the active hand
//! (lighting a welding tool), `--cast` casts the bot's spell LINE (from 0) at the first thing with that
//! sprite, `--wait` pauses.
//!
//! Output (stdout): one line per player, `player <client_id> <x> <y> [self]`, then one
//! line per notice received, `notice <message id>`.
//! Exit code 1 if the bot never received its own player.

use std::net::SocketAddr;
use std::time::Duration;

use bevy::log::LogPlugin;
use bevy::prelude::*;
use rs13_headless::{BotConfig, BotReport, Step, bot_app, unique_client_id};
use rs13_network::protocol::ActionInput;
use rs13_world::{Dir, Slot};

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

    let result = report.lock().unwrap().take().unwrap_or_default();
    for player in &result.players {
        let tag = if player.own { " self" } else { "" };
        println!(
            "player {} {} {}{tag}",
            player.client_id, player.pos.x, player.pos.y
        );
    }
    for notice in &result.notices {
        println!("notice {}", notice.id);
    }
    exit
}

fn parse_args() -> Result<BotConfig, String> {
    let mut config = BotConfig {
        server: SocketAddr::from(([127, 0, 0, 1], rs13_network::DEFAULT_PORT)),
        client_id: unique_client_id(),
        dir: None,
        hold: Duration::from_secs(3),
        examine: None,
        prefer_job: None,
        name: None,
        say: None,
        script: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--use-held" {
            config.script.push(Step::UseHeld);
            continue;
        }
        let value = args.next().ok_or_else(|| format!("{arg} needs a value"))?;
        match arg.as_str() {
            "--server" => config.server = value.parse().map_err(|e| format!("--server: {e}"))?,
            "--dir" => config.dir = Some(parse_dir(&value)?),
            "--examine" => config.examine = Some(value),
            "--name" => config.name = Some(value),
            "--say" => config.say = Some(value),
            "--use" => config.script.push(Step::UseOn(value)),
            "--pull" => config.script.push(Step::Pull(value)),
            "--cast" => {
                let (line, sprite) = value.split_once(':').ok_or("--cast: LINE:SPRITE")?;
                let line = line.parse().map_err(|e| format!("--cast: {e}"))?;
                config.script.push(Step::CastOn(line, sprite.to_owned()));
            }
            "--equip" => config
                .script
                .push(Step::Act(ActionInput::Equip(match value.as_str() {
                    "uniform" => Slot::Uniform,
                    "shoes" => Slot::Shoes,
                    "suit" => Slot::Suit,
                    "head" => Slot::Head,
                    "mask" => Slot::Mask,
                    "glasses" => Slot::Glasses,
                    other => return Err(format!("--equip: unknown slot {other}")),
                }))),
            "--wait" => config.script.push(Step::Wait(Duration::from_secs_f32(
                value.parse().map_err(|e| format!("--wait: {e}"))?,
            ))),
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
