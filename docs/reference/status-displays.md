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

- **Ordinary displays** show the evacuation shuttle's countdown in Luna ("ETL" over
  minutes and seconds), which is blank until a shuttle is called. There is no shuttle yet,
  so they stay blank, as Luna's do at round start.
- **Supply displays** show where the supply shuttle is: "SPPLY" over "AWAY".

## Differences from Luna

| Luna | Here |
|---|---|
| The communications console sets messages, alert pictures (red alert, lockdown, biohazard) or blank on every display, by radio | Not built: there is no communications console yet |
| The shuttle countdown, and the supply shuttle's "SHIP" and timer | There are no shuttles yet |
| Long messages scroll; the AI's faces; an EMP's blue screen | Not built |
