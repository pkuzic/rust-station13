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

The lobby also holds a character setup, kept by the client in `rs13-prefs.ron` beside its key: gender, hair and beard from Luna's
styles, their colour, eye colour, how dark the skin is, underwear, whether you'd like to be a traitor, and three jobs in order.
Those who'd like to be traitors are picked first. **Observe** enters as a ghost, with no character and no job, and **View the Crew
Manifest** lists who's aboard once anyone is.

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

## Red alert and the displays

With `heads` access the console also declares and cancels red alert, which turns every light on the station red and back.
Calling the pods declares it too. It sets what ordinary status displays show: the countdown to the pods ("ETL" over minutes
and seconds, then "ETA"), nothing, a message of two lines, or one of Luna's pictures (none, red alert, lockdown, biohazard).
Supply displays aren't changed. Long messages are cut to 5 characters a line, where Luna's scroll.

## Votes

Sources: `luna:code/game/vote.dm`, `config/config.txt`. Code: `apps/server/src/vote.rs`, `apps/client/src/vote_panel.rs`.
Tests: `apps/headless/tests/votes.rs`.

V opens the vote window, in the lobby or in the world, dead or alive. Anyone can start a vote to restart or, on a server
running rounds, to change the mode. It's open for a minute, and the next can start 10 minutes after it ends.

- **Restart** passes if more say yes than no. Those who don't vote don't count, so one yes and nobody against is enough.
  The server restarts 5 seconds later.
- **Mode** goes to whichever choice has the most, ties at random, and those who don't vote count as no change. Before the
  round it settles this round. Once a round is on, the server restarts in 10 seconds. The choice stays for the rounds
  after, until voted otherwise. **Secret** goes back to a random mode each round.

`rs13-server --vote-seconds N` shortens a vote, for trying it out.

## Extended and sandbox

Extended has nothing more to it than a word of welcome. Sandbox never comes up by chance (probability 0): it has to be voted for or
forced. In a sandbox round **F5** opens a panel that gives anyone a space suit, helmet, gas mask and jetpack (what they wore there
falls to the floor), 50 metal, 50 glass, an airlock, a canister, a toolbox, a first aid kit or any item at all, at their feet. An
admin can turn off the any-item list with `togglespawning`. The round doesn't end when the pods arrive: it runs until it's restarted.

## Differences from Luna

| Luna | Here |
|---|---|
| Preferences for each job at three levels, and the round divides people up with a random draw for each post | Three jobs you'd like in order, first come among those who chose |
| A character setup window: gender, hair, skin, eyes, and more | Only a name |
| An observe button, to watch as a ghost | Not built |
| A secret mode that hides what's been picked, and modes chosen by vote | Always announced; votes aren't built |
| A mode that can't start returns everyone to the lobby | Extended always can |
| A message of the day, a preferences file saved between rounds | Not built |
| Roles are given before anyone appears | Same, but with no antagonists yet |
| The pods fly: their contents move to a transit area and then CentCom, leaving hull behind | They stay where they are with their doors bolted, and whoever's aboard has escaped |
| The prison shuttle and its console | Not built |
| Sounds while launching | Only chat messages |
| Modes can forbid or delay the call | No mode needs that yet |
| Clients idle for 2 minutes don't count in a vote | They do |
| A voted mode is saved to disk | It lasts as long as `rs13-server` runs |
| Admins can start a custom vote | Not built: no admins yet (1.15) |
| The console has a message list, with printed reports and intercepts from Central Command, and an AI interface | Not built: nothing sends it messages yet, and there's no AI (3.9) |
