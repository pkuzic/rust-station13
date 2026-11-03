# Reference spec: speech and character names

Sources:
- `luna:code/modules/mob/living/say.dm`: `/mob/living/say`
- `luna:code/modules/mob/say.dm`: `say_quote`
- `luna:code/__HELPERS/text.dm`: `reject_bad_name`
- `luna:code/defines/world.dm`: `view = "15x15"`
- `luna:code/defines/global.dm`: `MAX_MESSAGE_LEN = 1024`

## Luna behaviour

**Saying.** The message is sanitised and cut to `MAX_MESSAGE_LEN` (1024) characters, then
trimmed. An empty message does nothing. The first letter is capitalised. Prefixes switch
to emotes (`*`), radio channels (`;`, `:x`) or whispering; these are not covered here.

**Who hears.** With no special mode, `message_range` is null, so `hearers()` uses the world
view: everyone who can see the speaker within a 15×15 square (7 tiles each way). Opaque
turfs and objects (walls, closed doors) block sight. The speaker always hears themselves.

**Format.** `say_quote` picks the verb from the last character: `?` gives "asks", `!` gives
"exclaims", anything else "says". Stuttering and brain damage override the verb ("stammers",
"gibbers"). Result: `John Smith says, "Hello."`

**Names (`reject_bad_name`).** At most 20 characters (26 in later code; we use **26**).
Allowed: Latin letters, and `' - .` and spaces between words. The first letter of each word
is forced to upper case, double spaces are collapsed, and a trailing space is removed.
Digits are dropped unless allowed (they aren't, for humans). Any other character rejects
the name. At least 2 letters are required. "space", "floor", "wall", "r-wall", "monkey",
"unknown" and "inactive ai" are refused.

## Implemented (`game/classic/src/speech.rs`)

| Behaviour | Status |
|---|---|
| Trim, limit to 1024 characters, ignore empty, capitalise the first letter | keep |
| Control characters removed | keep (sanitising) |
| Verb from the last character: ask / exclaim / say | keep |
| Hearers: Chebyshev range 7 with line of sight; speaker always hears | keep |
| Line of sight: a straight line (Bresenham) must not cross an opaque tile or object | **approximation**: BYOND's `view()` uses its own shadow casting, still to verify |
| Opaque: walls; airlocks while closed or opening (Luna sets opacity at the end of each animation) | keep |
| Name rules as above, max 26; invalid names keep the random default | keep |
| Players pick a name when they connect; otherwise a random English name | keep (Luna has a preferences screen; we don't yet) |
| Speech text is the player's own words in any language; only the frame ("says") is localised | keep |
| Examining a person: "That's John Smith." | keep |

## Whispers, emotes and OOC

Sources: `luna:code/modules/mob/living/carbon/human/whisper.dm`, `human/emote.dm`, `luna:code/modules/mob/say.dm`,
`luna:code/WorkInProgress/Uristqwerty/chat_tabs/ooc.dm`. Tests: `game/classic/tests/speech.rs`,
`apps/headless/tests/chat.rs`.

- **Whisper:** `:w text` (or `#w`, `.w`, and `:ц` on a Russian layout) is heard whole within 1 tile. Within 2, about half
  the words become "...". Within 5, you only see that they're whispering.
- **Emotes:** `*smile` and 40 others, or `*me` and your own words, which the M key types for you. Seen as far as speech
  is heard. An unknown one tells only the one who typed it.
- **OOC:** the O key, in the lobby or the world, alive or dead. Everyone on the server reads it. It shows the name of your
  character, or the name you typed in the lobby, or "Someone".
- **Dead chat:** ghosts talk among the dead, as `ghosts.md` says.

The chat shows in the lobby too, beside the job list.

| Luna | Here |
|---|---|
| OOC shows the player's account | There are no accounts |
| (Un)Mute OOC, and admins can turn it off | Not built |
| Emotes that take a target (`*bow Bob`, `*salute`, `*glare`, `*handshake`), `*signal`, and `*help` | Not built |
| `*faint` and `*collapse` knock you down, `*vomit`, needs like `*hungry` | Words only, or not built |
| Emotes are only in English keys | The same here, with the lines translated |
| A radio with its microphone on picks up whispers within a tile | It doesn't |
| Muzzles and mute stop whispers and emotes | They don't stop anything yet |

## Not implemented yet

Stuttering and brain damage, languages (understood or not), deafness, rate limiting, logging.
