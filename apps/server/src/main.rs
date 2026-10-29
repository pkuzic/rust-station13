//! Dedicated server: engine plus the classic game build.
//!
//! Usage: `rs13-server [--port N]`

use bevy::log::LogPlugin;
use bevy::prelude::*;

fn main() -> AppExit {
    let port = parse_port().unwrap_or(rs13_network::DEFAULT_PORT);
    let mut app = rs13_server::server_app(port);
    app.add_plugins(LogPlugin::default());
    app.run()
}

fn parse_port() -> Option<u16> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--port" {
            return args.next()?.parse().ok();
        }
    }
    None
}
