# Network protocol

Defined in `engine/network/src/protocol.rs`. Client and server must register the same
protocol; `rs13_network::PROTOCOL_VERSION` (currently **1**) is bumped on every
incompatible change and is part of the netcode protocol id, so mismatched builds cannot
connect.

## Transport

- Lightyear 0.31 over UDP, netcode.io connections (`docs/adr/0002-bevy-lightyear-pin.md`).
- Default port `13013` (`rs13_network::DEFAULT_PORT`).
- Fixed tick 50 ms (20 TPS) on both sides; the server replicates every tick.
- **Authentication is development-only:** a zero netcode key and client-chosen ids.
  Anyone can connect. Hub-issued connect tokens replace this before public servers.

## Replicated components (server → all clients)

| Component | On | Meaning |
|---|---|---|
| `MapLayer(TileMap)` | the map entity | One deck: size, tile palette, tiles, spawn points |
| `GridPos(TilePos)` | mobs | Tile the mob stands on |
| `Player { client_id }` | player mobs | Netcode id of the controlling client |

The controlling client also receives Lightyear's `Controlled` marker on its own mob.
Player mobs are despawned on the server when their client disconnects.

## Messages

| Message | Direction | Channel | Meaning |
|---|---|---|---|
| `MoveInput { dir: Option<Dir>, run: bool }` | client → server | `IntentChannel` (ordered, reliable) | The movement the player is holding. Sent when it changes; the server applies the latest one every tick until a new one arrives. |

## Rules

- The server is authoritative. Clients only send intentions; the server decides positions.
- Clients render `GridPos` with a short glide between tiles; there is no client-side
  prediction yet.
- Player-visible text will be sent as **message id + arguments**, never formatted
  strings, so each client renders it in its own locale.

## To define

- Interest management: which chunks and entities each client receives (today: everything).
- Chunked map replication (today: the whole deck in one component).
- Command validation and rate limits.
- Session handshake with player name and asset manifest hash.
- Asset manifest format (SHA-256 content addressing), see `MODDING.md`.
