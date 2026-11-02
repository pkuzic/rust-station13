# Smoking

Sources: `luna:code/game/objects/items/weapons/cigs_lighters.dm` (`cigpacket`, `matches`,
`match`, `lighter`, `cigarette`, `cigbutt`). Sold by the cigarette machine (`vending.md`).

Code: `game/classic/src/smoking.rs`. Tests: `game/classic/tests/smoking.rs`.

A lit match, lighter or cigarette is a small flame: every life cycle (2 s) it touches the
air where it is, on the floor or on whoever holds or wears it, at 700 K (Luna
`hotspot_expose(700, 5)`). That is hot enough to light plasma (`fire.md`).

## Cigarette packet (`cigarette-packet`)

Six cigarettes. With the packet in one hand, an empty hand clicking it takes one out into
that hand; with none left: "You're out of cigs, shit! How you gonna get through the rest of
the day...". Its sprite shows what is left once opened (`cigpacket5` … `cigpacket0`), and
examining it then says "There are 5 cigs left!".

## Matchbox and matches (`matchbox`, `match`)

A matchbox holds seven matches (and only matches). Clicking the box with an unlit match
strikes it (`match_lit`); a match isn't put back in. A match burns for 15 life cycles, then
is burnt out (`match_burnt`). Dropped while lit it looks burnt at once but burns on until
its time is up, as in Luna.

## Lighter (`lighter`)

A cheap lighter in one of four colours (Luna `lighter/random`). Used in the hand it lights:
"After a few attempts, Name manages to light the cheap lighter."; one time in five it burns
an arm for 2 as well ("You burn yourself while lighting the lighter."). Used again it goes
out: "Name quietly shuts off the cheap lighter.".

## Cigarette (`cigarette`)

Worn in the mask slot (`cigoff`, lit `cigon`, on the face too). Clicking an unlit cigarette,
or yourself while wearing one, with a lit welding tool, lighter, match or cigarette lights
it, and everyone sees how: "Name casually lights the cigarette with the welding tool, what a
badass.", "After some fiddling, Name manages to light their cigarette with the cheap
lighter." or "Name lights their cigarette with their match.". It burns for 150 life cycles
(5 minutes), then leaves a cigarette butt on the floor: "Your cigarette goes out.". Used in
the hand while lit, it is dropped and trodden out: "Name calmly drops and treads on the lit
cigarette, putting it out instantly.", leaving a butt.

## Differences from Luna

| Luna | Here |
|---|---|
| A lit lighter gives off a little light | Not built |
| A burnt-out match still lights cigarettes (it counts as lit) | It doesn't |
| Cigarettes carry reagents into the smoker, and plasma or fuel in one explodes when lit | Cigarettes hold nothing yet |
| Lighting someone else's cigarette, the zippo, cigars | Not built |
