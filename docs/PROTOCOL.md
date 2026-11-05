# Network protocol

Defined in `engine/network/src/protocol.rs`. Client and server must register the same
protocol; `rs13_network::PROTOCOL_VERSION` (currently **54**) is bumped on every
incompatible change and is part of the netcode protocol id, so mismatched builds cannot
connect.

## Transport

- Lightyear 0.31 over UDP, netcode.io connections (`docs/adr/0002-bevy-lightyear-pin.md`).
- Default port `13013` (`rs13_network::DEFAULT_PORT`).
- Fixed tick 50 ms (20 TPS) on both sides; the server replicates every tick.
- **Authentication is development-only:** a zero netcode key and client-chosen ids.
  Anyone can connect. Hub-issued connect tokens replace this before public servers.

## Replicated components (server → clients)

Entities on the map are sent only to the clients whose mob can see them
(`apps/server/src/sight.rs`, spec `docs/reference/lighting.md`); an entity that goes out of
sight is despawned on that client and spawned again when it comes back.

| Component | On | Meaning |
|---|---|---|
| `MapLayer(TileMap)` | the map entity | One deck: size, tile palette (each tile: id, `solid`, `opaque`, `airtight`), tiles, spawn points |
| `GridPos(TilePos)` | mobs | Tile the mob stands on |
| `Player { client_id }` | player mobs | Netcode id of the controlling client |
| `NetId(u64)` | everything clients see | Stable id; clients name entities by it |
| `Appearance { sprite, state }` | objects and mobs | Sprite / prototype / noun id and state, like BYOND `icon_state` |
| `Hidden` | pipes and cables under floor tiles | Not drawn and not clickable (BYOND `invisibility`) |
| `HandsView { left, right, active }` | mobs | Held items as `NetId`s and the active hand |
| `DisplayName(String)` | people | Character name (English only) |
| `AtmosView { pressure_dkpa, plasma_dmol, n2o_dmol }` | the map entity | Pressure of every tile, 0.1 kPa, for the F3 debug overlay, and its plasma and sleeping gas, 0.1 mol (saturating at 25.5), for the gas overlays; twice a second |
| `LightSource([u8; 3])` | lit light fixtures and flashlights on the floor; people carrying lit flashlights (summed) | Red, green and blue strength in tiles of reach; clients compute the light on each tile from these (`docs/reference/lighting.md`) |
| `Opaque` | closed airlocks | Blocks sight and light through its tile |
| `Silhouette` | stand-ins for people in the dark, with `GridPos`, `Facing` and `Appearance` only | Drawn black; the person themselves isn't sent to anyone else while they stand in the dark (`docs/reference/lighting.md`, Silhouettes) |

An item in a hand has no `GridPos`; removing the component replicates, and clients
stop drawing it on the map. The map changes when floor tiles are pried up or put back, and
clients redraw the tiles that changed.

The controlling client also receives Lightyear's `Controlled` marker on its own mob.
Player mobs are despawned on the server when their client disconnects.

## Messages

| Message | Direction | Channel | Meaning |
|---|---|---|---|
| `MoveInput { dir: Option<Dir>, run: bool }` | client → server | `IntentChannel` (ordered, reliable) | The movement the player is holding. Sent when it changes; the server applies the latest one every tick until a new one arrives. |
| `ActionInput` | client → server | `IntentChannel` | `UseOn(NetId)`, `Examine(NetId)`, `Drop`, `SwapHands`, `Say(String)`, `SetName(String)`, `SetIntent(Intent)`. Queued and applied on the next tick; unknown ids are ignored. `SetName` is accepted once per session; an invalid name keeps the random one and sends `name-rejected`. |
| `Notice(LocMsg)` | server → one client | `NoticeChannel` (ordered, reliable) | A message for that player: message id + arguments, formatted by the client in its own language. |
| `StatusUpdate { health, stat, intent, pressure_dkpa, alerts }` | server → one client | `NoticeChannel` | The player's own health, state, intent, the pressure where they stand and environment alerts (vacuum, low/high pressure, low oxygen, cold, hot), sent when they change. Nobody else's health is sent. |
| `PlaySound { sound, at, volume }` | server → one client | `SoundChannel` (unordered, unreliable) | A sound the player hears: sound id, source tile and loudness 1–100. The server decides who hears it and how loud (distance, walls, closed doors and edges, air); the client only pans it by the source's x offset. See `reference/sound.md`. |

## Rules

- The server is authoritative. Clients only send intentions; the server decides positions.
- Clients render `GridPos` with a short glide between tiles; there is no client-side
  prediction yet.
- Player-visible text is sent as **message id + arguments** (`LocMsg`), never formatted
  strings, so each client renders it in its own locale.

## To define

- Interest management: which chunks and entities each client receives (today: everything).
- Chunked map replication (today: the whole deck in one component).
- Command validation and rate limits.
- Session handshake with player name and asset manifest hash.
- Asset manifest format (SHA-256 content addressing), see `MODDING.md`.
