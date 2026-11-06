# Monkey and corporate restructuring rounds

Two of Luna's small modes (`gamemodes/monkey`, `gamemodes/restructuring`). Neither can be voted for
(Luna's `votable = 0`).

## Monkey

Luna's `config.txt` leaves it out, so it keeps the default chance of 1 when the round is picked at
random.

- 5 s into the round, with at least 3 players there, a third of the rest and one more (at most 4)
  become monkeys (Luna `monkeyize`): all they hold and wear falls to the floor, they take the monkey's
  look and the name "monkey", and they're told so.
- A monkey on harm intent bites instead of punching (Luna's `attack_paw`):
  - another monkey, three times in four, for 1 to 5 brute, or it misses;
  - a person, every time, for 1 to 3 brute. A space suit, a bio suit or armour keeps the teeth out.
    One time in four the bite is seen to miss; otherwise nothing happens at all.
- The round ends when the pods arrive, as most do. Monkeys win if a living monkey is aboard and no
  living person is. Then the players turned into monkeys are named. Otherwise the research staff has
  stopped the invasion.

Luna also gives every monkey jungle fever. It does nothing: the bite that would pass it on checks a
field the disease is never stored in. So no one turns from a bite, here or there.

## Corporate restructuring

Its chance is 0 in Luna's `config.txt`, so it only comes up when forced.

- As the round starts, head office announces a restructuring and says that layoffs will be handled
  with compassion.
- 12 s in, it names one living crew member at random, with their job when they have one, to be
  "terminated", and tells everyone. The one named is told to watch out. While nobody is alive to
  name, it looks again every 3 s.
- The round ends when the pods arrive.

Luna's `check_win` would name someone else when the target died, and declare a winner when one
person was left. Nothing calls it (the ticker's calls are commented out), so none of that happens.
Its zeroth law for the AI waits on the AI (3.9).
