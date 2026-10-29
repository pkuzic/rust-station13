# Testing

## Required before every merge

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same commands on Linux and Windows (`.github/workflows/ci.yml`).

## Kinds of tests

| Kind | Where | Purpose |
|---|---|---|
| Unit | `#[cfg(test)]` in each crate | Pure logic: coordinates, gas maths, formatting |
| Crate integration | `<crate>/tests/` | Public API contracts of one crate |
| Property | `proptest` in unit or integration tests | Invariants: conservation of gas moles and energy, containment never cyclic |
| Scenario | `tests/` driven by `rs13-headless` | Load a small map, run N ticks with scripted commands, assert world state |
| Benchmark | `benches/` (criterion) | Tick time for atmos, power and replication |

## Simulation regression

Every simulation bug fix adds a scenario that reproduces it. Scenarios are deterministic:
fixed seed, fixed tick count, no wall-clock time.

## Reference fidelity

Tests that check behaviour against Luna cite the spec, e.g.
`// spec: docs/reference/airlock.md, luna:code/game/machinery/doors/airlock.dm`.

## Performance budget

At 20 TPS a tick has 50 ms. Target for a full NSV Luna map with 15 players: total tick
time under 25 ms on a mid-range 4-core server.
