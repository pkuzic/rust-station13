# Revolution and RP revolution

Luna's `code/game/gamemodes/revolution`, in `game/classic/src/revolution.rs`.

## Setting up

Up to three of the crew lead it, picked first from those who'd like to be traitors, and never a head of staff, a security officer or a
forensic technician. Each is told to kill, or in RP to capture, every head of staff aboard: captain, head of security, head of personnel,
chief engineer and research director. In a plain revolution each leader is given a flash.

With nobody to lead it or nobody to overthrow, the round is Extended instead.

## Joining and leaving

- **Revolution**: a leader's flash in the face turns someone, if it blinds them. A blow to the head that knocks someone unconscious has an
  even chance of turning them back.
- **RP revolution**: any revolutionary can right-click someone within seven tiles and choose Rev-Convert. They're asked in a window, and
  can't be asked again for five seconds. There's no way back out.

The heads, security officers and forensic technicians can't be turned. Revolutionaries see a blue R over their leaders and a red R over each
other; nobody else sees them.

## How it ends

| Who wins | Revolution | RP revolution |
|---|---|---|
| The revolution | Every head is dead or gone | Every head is dead, gone, cuffed or out cold |
| The heads | Every leader is dead or gone | Every leader is dead, gone, cuffed or in the brig |

Either ends the round at once. If both would, the revolution wins, as in Luna. If the pods get home first, the round ends undecided.

An RP revolution names its leaders to everyone after 110 minutes.

## Differences from Luna

| Luna | Here |
|---|---|
| Turning someone back leaves them on the list of revolutionaries, so they're still in the final roll | They come off it |
| Icons in `update_all_rev_icons` have leaders red and followers blue, against what its own join message says | Leaders blue, as the message says |
| Only the first leader's objectives are checked for a win | They all have the same ones, so all the same |
| Fewer players than needed restarts the world, in code that can never run | The round is Extended |
| A second leader can be picked twice | Never |
| RP names its leaders on a paper at comms consoles at 90 minutes, then aloud at 110 | Aloud at 110 |
| An intercept report reaches the comms consoles after a few minutes | Not built |
| Leaders may get a flash, only if their pockets are full | They always do |
