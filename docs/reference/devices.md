# Devices, assemblies and tank bombs

Sources: `luna:code/game/objects/radio/signaler.dm`, `luna:code/game/objects/items/devices/`
(`timer.dm`, `proxy_sensor.dm`, `igniter.dm`), `luna:code/defines/obj/assemblies.dm`,
`luna:code/game/objects/weapons.dm` (the assemblies and tank bombs), `luna:code/game/objects/tank.dm`
(`tank/plasma/ignite`), `luna:code/game/objects/assemblies.dm` (the transfer valve). `NSV_Luna.dmm`:
signalers, timers, proximity sensors and igniters about the station, mostly in Toxins, Robotics
and the tool storages; seven timer-igniter assemblies in Chemistry and Toxins; ten transfer
valves.

Code: `game/classic/src/devices.rs`. Tests: `game/classic/tests/devices.rs`.

Luna counts timers in its master controller's cycles, about two seconds each; here they are
machine cycles of 2 s. Windows are the list window: each line is a button.

## The devices

- **Remote signaler** (`signaler`). Frequency 144.1–148.9, odd tenths only, 145.7 to start; code
  1–100, 30 to start. Its window: Send signal, the frequency with steps of 0.2 and 1.0, and the
  code with steps of 1 and 5. A signal reaches **every other signaler on the same frequency and
  code, wherever it is**; each beeps and sets off what it's part of. A signaler sends at most
  every half second. A screwdriver opens it so it can be attached ("The radio can now be attached
  and modified!") or closes it. The autolathe makes them.
- **Timer** (`timer`). Its window: start or stop, the time left as m:ss, and steps of 1 and 30, up
  to 10:00. Started, it counts down a cycle at a time and goes off one cycle after reaching 0,
  then stops. A loose timer beeps.
- **Proximity sensor** (`prox-sensor`). Its window arms and disarms it, at once (Luna's arming
  delay never worked). Armed and lying on the floor, or in something lying on the floor, it goes
  off whenever someone steps onto or next to its tile; dropped while armed, it goes off. A loose
  one beeps.
- **Igniter** (`igniter-device`). Used in the hand, while ready, it sparks at 1000 K where it is:
  that lights the air if there's plasma and oxygen in it, and does nothing else. A screwdriver
  switches it between ready ("The igniter is ready!") and attachable.

## Assemblies

Using one device on another, held or on the floor within reach, puts them together in the hand
that held the first:

| Used | On | Makes | Sets off |
|---|---|---|---|
| An open signaler | An attachable igniter | Radio-igniter | Its signal sparks the igniter |
| A timer | An attachable igniter | Timer-igniter | Running out sparks the igniter |
| A proximity sensor | An attachable igniter | Proximity-igniter | A step sparks the igniter |
| An open signaler | A timer | Signaler-timer | Running out sends the signal |
| An open signaler | A proximity sensor | Signaler-proximity sensor | A step sends the signal |

Used in the hand, an assembly shows its trigger's window, and the signaler's as well where it has
one. A screwdriver secures it (the igniter becomes ready, a signaler closes) or unsecures it; only a
secured igniter assembly sparks. A wrench takes an unsecured assembly apart, both parts falling
where it is. The timer-igniters on the map start unsecured. Luna's signaler-timer and
signaler-sensor assemblies could never send (two bugs). Here they work as they were meant to.

## Tank bombs

A **secured** timer-, proximity- or radio-igniter used on a plasma tank mounts it: a bomb, in the
hand that held the assembly. A welding tool (Luna doesn't check it's lit) bores the tank's valve
("A pressure hole has been bored to the plasma tank valve. The plasma tank can now be ignited.") or
closes it again. A wrench takes an unwelded bomb back off its tank.

When its trigger goes off, it beeps, and then:

- **Unwelded**: the tank's gas is let out where the bomb lies. Held, it's lost. A proximity bomb
  disarms.
- **Welded**: nine times in ten the tank goes up; otherwise nothing happens and it can go off again.
  Going up, its fuel is the plasma and a sixth of the oxygen in it. Our tanks hold twice the moles
  Luna's do (its gas constant is doubled), so the fuel is halved to match its blasts. By the tank's
  temperature:

  | Tank | Strength s | Devastation, heavy, light, flash |
  |---|---|---|
  | Over 400 °C | fuel / 10 | s, 2s, 3s, 4s |
  | Over 250 °C | fuel / 15 | 0, s, 2s, 3s |
  | Over 100 °C | fuel / 20 | 0, 0, s, 3s |
  | Cooler | | No blast: the gas let out and sparked at 1000 K |

  A tile is caught while its distance is under each range, as in Luna's small explosions. A tank
  straight off the shelf is cool, so it only lets its plasma out and lights it. The tank and bomb
  are gone either way.

## The tank transfer valve (`transfer-valve`)

Holds two tanks and one device. A tank used on it goes on ("You attach the tank to the transfer
valve."; a third: "There are already two tanks attached, remove one first!"). A signaler, timer or
proximity sensor used on it goes on its controls; set it up first, as it can't be reached once
on. Its window lists the two tanks and the device, each taken off onto the floor by its line, and
the valve, opened or closed by its line while it's held.

When its device goes off, the valve is worked, and then the device can't work it again for five
seconds. Opening it with both tanks on pours all of the first tank's gas into the second, which
takes in the first one's volume as well. What then happens in the second tank is what happens in
any tank (`explosions.md`): it burns, and too much pressure makes it leak, rupture or explode.

## Not built

- Signaler wires, and signalers attached to airlock wires (4.1).
- The infrared emitter, its beam and the infrared sensor: neither emitter nor beam is on Luna's
  station, and the sensor only shows beams.
- Mousetraps: Luna's station has none, and they can only be folded out of cardboard, which isn't
  built.
- The health-analyser-igniter and the armour bomb vest, and the flamethrower.
- The admin log of welded bombs.
- Grenades and implants come separately.
