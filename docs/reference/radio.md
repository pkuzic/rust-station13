# Reference spec: radios and intercoms

## What Luna does

Sources: `luna:code/game/objects/radio/radio.dm`, `luna:code/game/objects/radio/intercom.dm`,
`luna:code/defines/obj/radio.dm`, `luna:code/modules/mob/living/say.dm`.

- **Radios:** a handheld radio ("station bounced radio", w_class 2, worn on the belt) starts on the common channel, 1459, shown as 145.9. Its speaker is on and its microphone off.
- **Intercoms:** wall-mounted radios on the same channel. They need no power in the compiled code.
- **Talking into one:** a prefix of `:`, `#` or `.` followed by a key sends speech into a radio. The same keys work on a Russian layout.
  - `r` uses the radio in the right hand and `l` the one in the left.
  - `i` uses every intercom within one tile.
  - Speech sent this way is heard directly only by those within one tile (`message_range = 1`), in italics.
- **Microphone engaged:** a radio passes on everything said within `message_range` of it (`hear_talk`).
- **Receiving:** `talk_into` sends to every radio on the frequency, the speaker's own included. Each one with its speaker on plays the message to the mobs that can see it:
  - within 1 tile for a handheld radio;
  - within 7 tiles for an intercom.
  - A mob hears each message once, even near several radios.
- **Format:** `John Smith [145.9] says, "Hello."` For department frequencies the number becomes a name ("Engineering").
- **Radio window:** microphone and speaker switches, frequency buttons, and wires behind a screwdriver.
- **Quirk:** a radio plays at most two messages a second (`TRANSMISSION_DELAY`) and drops the rest.

## What we build (`game/classic/src/radio.rs`, `speech.rs`)

| Item | Ours | From Luna |
|---|---|---|
| Radio, intercom | handheld `radio` (belt) and wall `intercom`, both on 145.9, speaker on, microphone off | keep |
| Prefixes | `:r` `:l` `:i` (and `#` `.`, and к д ш on a Russian layout); heard directly within one tile | keep; other channels have no headsets yet |
| Microphone on | passes on what is said within the speech's range of it, if it can see the speaker. Switched by an empty hand on an intercom or radio on the floor, or by clicking a radio in hand | **simplified**: Luna's window. A held radio's microphone doesn't hear its holder, as in Luna. |
| Receiving | every listening radio on the frequency plays it within 1 tile (radio) or 7 (intercom) in sight; a mob hears each message once | keep |
| Format | `[145.9] John Smith says, "Hello."` | keep, with the frequency first |
| Speaker switch, frequency, wires, two-a-second limit | none | need the radio window; the limit drops messages |
| Headsets | the engineering headset, worn on the ears: `;` talks into it (heard directly within a tile), and it plays the common channel to its wearer and anyone beside them. Engineer spawn points get one, as Luna's engineers start with one | keep |
| Department channels | none | Luna's humans can't use them either: its say code only handles `:e` and the rest for robots |
