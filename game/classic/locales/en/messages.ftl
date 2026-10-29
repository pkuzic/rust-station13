# Args: $user, $target, $user_gender, $weapon (noun id).
attack-with = { $user } hits { $target } with the { NOUN($weapon) }.

# Args: $item (noun id).
item-fell = The { NOUN($item) } { GENDER($item) ->
        [plural] fall
       *[other] falls
    } to the floor.

# Args: $item (noun id), $count (number).
stack-count = { $count } { $count ->
        [one] { NOUN($item) }
       *[other] { NOUN($item, number: "pl") }
    }

# Args: $user, $user_gender, $item (noun id), $target (noun id).
use-on = { $user } opens the { NOUN($target) } with the { NOUN($item) }.

# Args: $item (noun id).
examine = That's { NOUN($item, case: "indef") }.

airlock-pushed = The airlock forces you out of the way!

# Hands bar labels.
hud-hand-left = L
hud-hand-right = R

# Args: $name (English, never declined).
examine-person = That's { $name }.

# Args: $name, $verb (say/ask/exclaim), $text (the player's own words, any language).
say = { $name } { $verb ->
        [ask] asks
        [exclaim] exclaims
       *[say] says
    }, "{ $text }"

# Args: $name (the name the player keeps instead).
name-rejected = That name is not allowed. You are { $name }.

# Prompt of the say line.
hud-say = Say:
