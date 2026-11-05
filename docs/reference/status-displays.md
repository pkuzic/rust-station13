# Status displays

Sources: `luna:code/game/machinery/status_display.dm` (`process`, `update`,
`update_display`, `texticon`, `get_shuttle_timer`, `receive_signal`); `NSV_Luna.dmm`: 24
kinds of placement, two of them supply displays (`mode = 4`).

Code: `game/classic/src/status_displays.rs`. Tests: `game/classic/tests/paperwork.rs`.

A frame on the wall (`frame`) showing up to two lines of five characters, drawn with Luna's
glyphs (`0`–`9`, `A`–`Z` and a few signs): right to left from 23 pixels, 5 pixels apart,
two lines at 9 and 17 pixels down, or one at 13. Each glyph is a small object of its own
(`status-glyph`) with that pixel offset. Without power the glyphs go and the frame stays
(10 W).

- **Ordinary displays** show the countdown to the escape pods, or what a communications console sets (`round.md`).
- **Supply displays** show where the supply shuttle is: "SPPLY" over "AWAY".

## Differences from Luna

| Luna | Here |
|---|---|
| The supply shuttle's "SHIP" and timer | There's no supply shuttle yet |
| Long messages scroll, where these are cut to 5 characters; the AI's faces; an EMP's blue screen | Not built |
