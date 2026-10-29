//! Server endpoint: listens for clients and replicates the world to them.

use core::net::{Ipv4Addr, SocketAddr};

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;

use crate::protocol::ProtocolPlugin;
use crate::{DEV_PRIVATE_KEY, NETCODE_PROTOCOL_ID, TICK_DURATION};

/// Lightyear server plugins, the shared protocol and per-client setup.
pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ServerPlugins {
            tick_duration: TICK_DURATION,
        });
        // The protocol must be registered after the Lightyear plugins.
        app.add_plugins(ProtocolPlugin);
        app.insert_resource(ReplicationMetadata::new(TICK_DURATION));
        app.add_observer(on_new_link);
    }
}

/// Give every incoming connection a replication sender.
fn on_new_link(trigger: On<Add<LinkOf>>, mut commands: Commands) {
    commands.entity(trigger.entity).insert(ReplicationSender);
}

/// Spawn the server endpoint and start listening on UDP `port`.
pub fn start_server(commands: &mut Commands, port: u16) -> Entity {
    let addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), port);
    let server = commands
        .spawn((
            Name::new("Server"),
            Endpoint::new(None),
            NetcodeServer::new(NetcodeConfig {
                protocol_id: NETCODE_PROTOCOL_ID,
                private_key: DEV_PRIVATE_KEY,
                ..Default::default()
            }),
            LocalAddr(addr),
            ServerUdpIo,
        ))
        .id();
    commands.trigger(Start { entity: server });
    server
}
