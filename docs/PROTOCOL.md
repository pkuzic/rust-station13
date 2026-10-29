# Network protocol

Status: placeholder. Filled in during the engine-foundation milestone.

## Fixed so far

- Transport: Lightyear 0.31 over UDP with netcode authentication
  (`docs/adr/0002-bevy-lightyear-pin.md`).
- Default port: `13013` (`rs13_network::DEFAULT_PORT`).
- `rs13_network::PROTOCOL_VERSION` is bumped on every incompatible change. Mismatched
  clients and servers refuse to connect with a clear message.
- The server is authoritative. Clients send **intentions** (move direction, use held item
  on target, open UI, say text); the server validates them and replicates the result.
- Chat and examine text are sent as **message id + arguments**, never formatted strings,
  so each client renders in its own locale.

## To define

- Session handshake: protocol version, server identity, asset manifest hash.
- Interest management: which chunks and entities each client receives.
- Replicated component set and delta encoding.
- Command validation rules and rate limits.
- Asset manifest format (SHA-256 content addressing), see `MODDING.md`.
