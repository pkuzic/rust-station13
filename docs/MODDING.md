# Modding

Status: design outline. Implementation starts after the engineering chamber milestone.

Goal: one universal client joins different community servers and downloads validated
assets automatically, while custom gameplay code runs only on the server.

## Mod classes

| Class | Runs / delivered |
|---|---|
| Sprites, audio, maps | Client downloads hash-addressed assets after verification |
| Prototypes, UI descriptions, locales | Data only; validated against schemas |
| Gameplay scripts | Server-side WASM (Wasmtime) with capability, fuel and memory limits |
| Native server mods | Trusted Rust crates compiled into a custom server build |
| Native client code | **Never** downloaded or executed from community servers |

## Security model

- TLS for downloads; SHA-256 manifest hashes for integrity; signed manifests where
  identity matters. A valid hash proves integrity, not harmlessness.
- Size quotas, decompression limits and restrictive parsers for every asset format.
- Per-server namespaces for downloaded content.
- WASM scripts get only the host capabilities they declare.
- The hub is optional; it is never a single point of failure.

## Licensing of mods

The server is AGPL-3.0: operators who modify the server and run it publicly must
offer their source. Data packs and assets are licensed by their authors.
