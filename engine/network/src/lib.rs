//! Network protocol, sessions and replication. Built on Lightyear; see
//! `docs/PROTOCOL.md` and `docs/adr/0002-bevy-lightyear-pin.md`.

pub use lightyear;

/// Bumped on every incompatible protocol change. Clients and servers with
/// different versions refuse to connect.
pub const PROTOCOL_VERSION: u32 = 0;

/// Default UDP port for game servers.
pub const DEFAULT_PORT: u16 = 13013;
