# Admins

Sources: `luna:code/modules/admin/admin_verbs.dm`, `admin.dm`, `IsBanned.dm`, `verbs/adminhelp.dm`, `verbs/adminjump.dm`,
`verbs/randomverbs.dm`, `luna:code/WorkInProgress/NewBan.dm`, `luna:code/game/cellautomata.dm` (`load_admins`). Code:
`apps/server/src/admin.rs`. Tests: `apps/headless/tests/admin.rs`.

## Who's an admin

As in Luna, the server has a list of keys and ranks. It's `config/admins.txt`, under where the server runs, in Luna's format
(`;` for a comment), and read when the server starts or restarts:

```
; key - rank
3fa94c01b2d6e87a - Host
```

The ranks are Luna's: Host, Coder, Super Administrator, Primary Administrator, Administrator, Secondary Administrator,
Moderator.

Luna's keys are BYOND accounts. There are none here, so the first time a client runs it makes a long random secret, and
keeps it in `rs13-key.txt` in the user's application data folder (`--key-file PATH` for another place). It sends it on
connecting, and the server's key for that player is the start of its SHA-256 hash. To find yours, press F2 and type `key`, or
read the server's log. Whoever has the secret file is that player, so it shouldn't be shared. Deleting it makes a new key.

`config/admins.txt` and `data/bans.txt` are not in the repository.

## The admin line

F1 asks the admins for help: every admin online sees it. F2 is the admin line, where Luna's verbs are typed as words:

| Lowest rank | Verbs |
|---|---|
| Moderator | `help`, `who` (names, keys, jobs, addresses), `asay`, `dsay`, `pm`, `announce`, `startnow`, `toggleooc`, `jump X Y [Z]`, `jumpto WHO`, `kick WHO [WHY]`, `ban WHO MINUTES [WHY]` (0 for good), `unban KEY`, `bans` |
| Secondary Administrator | `restart` |
| Administrator | `mute WHO`, `explode SIZE`, `emp HEAVY LIGHT` |
| Super Administrator | `get WHO`, `spawn THING`, `delete THING` (on your tile), `god WHO`, `rejuv WHO` |

WHO is the start of a name, or a key. No one can kick, ban or mute an admin of their own rank or higher. A kicked or banned
client doesn't try to reconnect by itself.

## Bans

A ban holds the key and the address it was seen at, and either can keep someone out. They're saved in `data/bans.txt`, and last
through restarts. Someone banned is told why as they connect and is dropped.

## Differences from Luna

| Luna | Here |
|---|---|
| Keys are real accounts that BYOND vouches for | A secret on the player's computer: there's nothing to stop someone making a new key, though the address ban still holds |
| Admins and bans are in a database, with its log | Plain files, and the server log |
| Verbs in a tab, and a player panel | Typed words on the F2 line |
| Banning by `computer_id` | There's no such thing here |
| About 100 verbs: variable editing, thunderdome, prison, sector doors, AI laws, secrets and more | The ones above. The rest wait for what they act on, or for a need |
| Job bans | Not built |
| Admins can go through a list of mobs by clicking | By name |
