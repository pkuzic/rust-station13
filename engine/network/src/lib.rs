//! Network protocol, sessions and replication. Built on Lightyear; see
//! `docs/PROTOCOL.md` and `docs/adr/0002-bevy-lightyear-pin.md`.

#[cfg(feature = "client")]
pub mod client;
pub mod protocol;
#[cfg(feature = "server")]
pub mod server;

use core::time::Duration;

pub use lightyear;

/// Bumped on every incompatible protocol change. Clients and servers with
/// different versions refuse to connect.
pub const PROTOCOL_VERSION: u32 = 46;

/// Default UDP port for game servers.
pub const DEFAULT_PORT: u16 = 13013;

/// Length of one fixed simulation tick, shared by client and server.
pub const TICK_DURATION: Duration = Duration::from_millis(1000 / rs13_core::TICK_RATE_HZ as u64);

/// Netcode protocol id: connections are refused unless both sides agree.
const NETCODE_PROTOCOL_ID: u64 = 0x5253_3133_0000_0000 | PROTOCOL_VERSION as u64;

/// Development-only netcode key. Anyone can connect with it. Real authentication
/// (hub-issued connect tokens) replaces this before public servers exist.
const DEV_PRIVATE_KEY: [u8; 32] = [0; 32];
