# Conveyor belts

Sources: `luna:code/WorkInProgress/recycling/conveyor.dm` (built into Luna: `conveyor/New`,
`update`, `process`, `attackby`, `attack_hand`; `conveyor_switch/attack_hand`, `process`;
`oneway`); `NSV_Luna.dmm`: belts in cargo, disposals and robotics, and the always-running
`centcom_auto` ones, with their switches.

Code: `game/classic/src/conveyors.rs`. Tests: `game/classic/tests/conveyors.rs`, and unit
tests there.

## Belt (`conveyor`)

A belt has a facing from the map, and switches find it by `id`. The facing gives the way it
carries things running forward and the way running backward. A diagonal facing is a
corner that turns things round:

| Facing | Forward | Backward |
|---|---|---|
| north, south, east, west | that way | the opposite way |
| north-east | east | south |
| north-west | south | west |
| south-east | north | east |
| south-west | west | north |

Every machine cycle (2 s), a running belt with power carries up to ten loose things lying
on it one tile on: items, people, closets, chairs, unbolted machines, anything that can be
pulled. Walls, dense things, people, shut edges (firedoors, window panes, window doors)
stop them. Everything that moves in a cycle is picked before anything moves, so nothing
rides two belts in one cycle. A running belt draws 100 W, and without power it stops.
Its sprite is `conveyor0`, `conveyor1` or `conveyor-1` in its facing.

`centcom_auto` belts run forward from round start.

**Clicking a belt** with an item puts the item down on it. With an empty hand, what you are
pulling is moved a step onto it and you let go.

## Switch (`conveyor-switch`)

Clicked with an empty hand. An ordinary switch goes from off to forward the first time,
then off, then reverse, then off, and so on: off always goes the other way to the last
run. A one-way switch (`oneway`) only goes forward and off. Every switch with the same id
shows the same, and the belts follow on the next cycle. Its sprite is `switch-off`,
`switch-fwd` or `switch-rev`.

## Differences from Luna

| Luna | Here |
|---|---|
| Broken belts stop the rest of their run | Nothing breaks belts yet |
| A person buckled to a belt lies on it | Nothing buckles to belts yet |
| Cyborgs can't put their tools on belts | There are no cyborgs yet |
