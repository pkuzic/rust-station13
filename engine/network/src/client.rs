//! Client endpoint: connects to a server and receives the replicated world.

use core::net::{Ipv4Addr, SocketAddr};

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use lightyear::netcode::client_plugin::NetcodeConfig;
use lightyear::prelude::client::*;
use lightyear::prelude::*;

use crate::protocol::ProtocolPlugin;
use crate::{DEV_PRIVATE_KEY, NETCODE_PROTOCOL_ID, TICK_DURATION};

/// Lightyear client plugins and the shared protocol.
pub struct ClientNetworkPlugin;

impl Plugin for ClientNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ClientPlugins {
            tick_duration: TICK_DURATION,
        });
        // The protocol must be registered after the Lightyear plugins.
        app.add_plugins(ProtocolPlugin);
    }
}

/// Spawn a client endpoint and connect to `server_addr` as `client_id`.
pub fn connect(commands: &mut Commands, server_addr: SocketAddr, client_id: u64) -> Result<Entity> {
    let auth = Authentication::Manual {
        server_addr,
        client_id,
        private_key: DEV_PRIVATE_KEY,
        protocol_id: NETCODE_PROTOCOL_ID,
    };
    let netcode = NetcodeClient::new(
        auth,
        NetcodeConfig {
            client_timeout_secs: 3,
            token_expire_secs: -1,
            ..Default::default()
        },
    )?;
    let client = commands
        .spawn((
            Name::new("Client"),
            Client,
            ReplicationReceiver,
            Link::default(),
            LocalAddr(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0)),
            PeerAddr(server_addr),
            netcode,
            UdpIo::default(),
        ))
        .id();
    commands.trigger(Connect { entity: client });
    Ok(client)
}
