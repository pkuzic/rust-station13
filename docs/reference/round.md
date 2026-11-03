# The round and the lobby

Sources: `luna:code/game/gamemodes/gameticker.dm` (`pregame`, `setup`), `game_mode.dm`, `luna:code/datums/configuration.dm`
(`pick_random_mode`), `luna:config/config.txt`, `luna:code/modules/mob/new_player/new_player.dm`.

Code: `game/classic/src/round.rs`, `apps/server/src/lobby.rs`, `apps/client/src/lobby_panel.rs`. Tests:
`apps/headless/tests/round.rs`.

`rs13-server --map luna-station` runs rounds. Any other map starts people straight in the world, unless given
`--rounds`, and `--no-rounds` turns them off. `--pregame SECONDS` sets the wait.

## The lobby

Connecting puts you in the lobby, with no body, on a black screen, and the server sends you nothing of the
station. Before the round you can declare yourself ready, see how long is left and how many are ready, type a name
with T, and choose the job you'd like, with those that have no free post greyed out.

## Starting

After 180 seconds a mode is picked at random, weighted by Luna's `config.txt`, and everyone ready is put in the
world, those with a job in mind first, in that job if a post is free and otherwise as in `jobs.md`. Someone not
ready stays in the lobby, where **Join the game** puts them in at once. The chat says which mode it is.

Only **extended** is built so far, with nothing special going on, so it's always that one for now.

## Differences from Luna

| Luna | Here |
|---|---|
| Preferences for each job at three levels, and the round divides people up with a random draw for each post | One job you'd like, first come among those who chose |
| A character setup window: gender, hair, skin, eyes, and more | Only a name |
| An observe button, to watch as a ghost | Not built |
| A secret mode that hides what's been picked, and modes chosen by vote | Always announced; votes aren't built |
| A mode that can't start returns everyone to the lobby | Extended always can |
| A message of the day, a preferences file saved between rounds | Not built |
| Roles are given before anyone appears | Same, but with no antagonists yet |
| The round ends when the shuttle reaches CentCom, and the server restarts | Not built: the escape shuttle is next (`LUNA_PARITY.md`) |
