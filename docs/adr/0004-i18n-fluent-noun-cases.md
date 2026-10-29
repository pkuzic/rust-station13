# 0004: Fluent localisation with a noun lexicon for case, number and gender

- Status: accepted
- Date: 2026-10-29

## Context

Russian and English ship from day one. Russian needs six grammatical cases, three
plural categories for counting, and verb agreement with the gender of both the speaker
and the object. SS13 messages constantly combine a player, a tool and a target
("X hits Y with the crowbar").

## Decision

- Use Fluent (`fluent-bundle` 0.16, concurrent bundle) for all messages.
- Keep a per-locale **noun lexicon** (`nouns.ron`) keyed by prototype id, holding gender
  and the case forms in singular and plural.
- Expose it to messages through two custom Fluent functions, `NOUN($id, case:, number:)`
  and `GENDER($id)`, so translators choose the case inside the message and code never
  has to know about Russian grammar.
- The server sends message ids and arguments; clients format locally.
- Player and character names are English-only and are never declined.

Details and examples: `docs/LOCALIZATION.md`.

## Alternatives rejected

- Fluent terms with a `$case` parameter: works for fixed nouns, but the noun in most
  messages is a runtime value (whatever item is in hand), which terms cannot express.
- Rule-based automatic declension: unreliable for compound names ("лист металла") and
  unnecessary since every noun is authored once.
