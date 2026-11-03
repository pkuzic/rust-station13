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

**Extended** and **traitor** (`traitor.md`) are built. `rs13-server --mode ID` always runs that one.

## Getting off the station

Sources: `luna:code/datums/shuttle_controller.dm` (`PodControl`, `CreateShuttles`),
`luna:code/game/machinery/computer/communications.dm`. Code: `game/classic/src/escape.rs`,
`apps/client/src/comms_panel.rs`. Tests: `game/classic/tests/escape.rs`.

Luna's way off is two escape pods, whose doors stay bolted until they're called. With `heads` access, a
communications console calls them. Everyone is told, the doors unlock, and they launch in 10 minutes, with a warning
every minute, then every 10 seconds, then every 5. The launch can be stopped until 30 seconds before.

At the launch their doors swing shut and bolt. 2.5 minutes later the pods arrive and the round is over: everyone is
told who escaped, who was left behind and who died. 60 seconds after that the server starts again with a clean
station, and clients reconnect and find themselves in the lobby.

`rs13-server --pod-seconds N` shortens the launch and the trip, for trying it out.

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
| The pods fly: their contents move to a transit area and then CentCom, leaving hull behind | They stay where they are with their doors bolted, and whoever's aboard has escaped |
| The prison shuttle and its console | Not built |
| Red alert in every area and sounds while launching; status displays show it | Only chat messages |
| Modes can forbid or delay the call | No mode needs that yet |
| The console also has messages, alert levels and status display settings | Only the call and the stop |
