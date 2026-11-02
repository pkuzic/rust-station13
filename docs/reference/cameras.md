# Security cameras

Sources: `luna:code/game/machinery/camera.dm` (`computer/security`: `attack_hand`,
`check_eye`; `camera`: `attackby`; `camera_sort`), `luna:code/defines/obj/machinery.dm`
(`camera`); `NSV_Luna.dmm`: 148 cameras and 3 security camera consoles on the decks
imported.

Code: `game/classic/src/cameras.rs`; the view in `apps/server/src/sight.rs` and the
client's `shadows.rs`, `lighting.rs` and `main.rs` (`follow_own_player`, `show_own_deck`);
the window in `apps/client/src/camera_panel.rs`. Tests: `game/classic/tests/cameras.rs`.

## Camera (`camera`)

Up on the wall facing its way (Luna `layer = 5`: above people), with Luna's animated
`camera` sprite. Each has a name tag (`c_tag`) and a network; every one on NSV Luna is on
`Luna`. **Wirecutters** switch it off (`camera1`) or on, which everyone sees: "Name has
deactivated the security camera!" / "…reactivated…". Anyone watching it loses the view:
"The screen bursts into static.".

## Security camera console (`security-console`)

A dense computer (`seccam`, `seccam0` without power; 250 W, a faint screen glow). Clicking
one within reach, while it has power, opens its window: the cameras on its network with a
name, sorted by name, each "(Deactivated)" when off, with **View** for the ones that work;
**Cancel** stops watching; **Close** closes it.

**Watching:** you see from the camera: the server sends what it sees instead of what you
see, all the way round (no blind spot behind you), and your screen centres on it, with its
deck, its light and its line of sight. The view ends, as Luna's `check_eye` ends it, when
you step out of reach of the console, fall unconscious, the console loses power, the camera
is switched off, or you close the window.

## Differences from Luna

| Luna | Here |
|---|---|
| Unnamed cameras are listed under an empty name (and overwrite each other) | Left out |
| The AI watches through cameras, follows people, and is told when one is cut | There is no AI yet |
| Holding paper up to a camera shows it to whoever watches; camera bugs | Not built |
| The camera list is a pick-one box that reopens after each pick | A window that stays open |
| The console uses 50 W per switch | It draws 250 W while it has power |
