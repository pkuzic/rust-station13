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

## Not implemented yet

Emotes, headsets (radios and intercoms are in `radio.md`), whispering, stuttering and brain damage, languages
(understood or not), deafness, rate limiting, logging.
