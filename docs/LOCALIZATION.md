# Localisation

Russian and English ship together from day one. Implementation: `engine/i18n`
(`rs13-i18n`), built on [Fluent](https://projectfluent.org/). Decision record:
`docs/adr/0004-i18n-fluent-noun-cases.md`.

## Rules

1. No player-visible text is hard-coded in Rust. Use message ids.
2. The server sends a message id plus arguments; each client formats it in its own
   locale. Two players can see the same event in different languages.
3. Every message exists in every shipped locale (checked by
   `game/classic/tests/locales.rs`).
4. Player and character names are English-only and passed as plain strings. They are
   never declined.

## Files

```
game/classic/locales/<locale>/messages.ftl   messages
game/classic/locales/<locale>/nouns.ron      noun lexicon, keyed by prototype id
```

## Nouns: case, number and gender

Each noun is defined once per locale. Russian nouns list all six cases in the singular,
and in the plural when the noun can be counted:

```ron
"crowbar": (
    gender: masculine,
    sg: (nom: "лом", gen: "лома", dat: "лому", acc: "лом", ins: "ломом", prep: "ломе"),
    pl: Some((nom: "ломы", gen: "ломов", dat: "ломам", acc: "ломы", ins: "ломами", prep: "ломах")),
),
```

`gender` is one of `masculine`, `feminine`, `neuter`, `plural` (pluralia tantum such as
"кусачки"). English nouns need only `nom`, plus plural `nom` for countable nouns.

Animacy is not modelled separately: the accusative is written out explicitly.

## Using nouns in messages

Two functions are available in every message:

| Function | Returns |
|---|---|
| `NOUN($id)` | nominative singular |
| `NOUN($id, case: "ins")` | the given case: `nom gen dat acc ins prep` |
| `NOUN($id, case: "gen", number: "pl")` | plural form |
| `GENDER($id)` | `masculine` / `feminine` / `neuter` / `plural`, for selectors |

```ftl
attack-with = { $user } { $user_gender ->
        [female] ударила
       *[other] ударил
    } { $target } { NOUN($weapon, case: "ins") }.

item-fell = { NOUN($item) } { GENDER($item) ->
        [feminine] упала
        [neuter] упало
        [plural] упали
       *[masculine] упал
    } на пол.

stack-count = { $count } { $count ->
        [one] { NOUN($item) }
        [few] { NOUN($item, case: "gen") }
       *[many] { NOUN($item, case: "gen", number: "pl") }
    }
```

Numbers select CLDR plural categories automatically (1 лом, 3 лома, 5 ломов, 21 лом).

## Missing translations

- A missing case falls back to the nominative of the same number. `Lexicon::missing_cases`
  reports gaps, and the classic build's tests require none for Russian.
- An unknown noun id renders as `{id}` so it is visible in game and gets reported.
