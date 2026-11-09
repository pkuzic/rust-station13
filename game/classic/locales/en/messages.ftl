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

# Combat. Args: $attacker, $victim (names), $item (noun id), $zone (body part noun id).
attacked-with = { $victim } has been attacked in the { NOUN($zone) } with the { NOUN($item) } by { $attacker }!
punched = { $attacker } has punched { $victim }!
punched-face = { $attacker } has punched { $victim } in the face!
punch-missed = { $attacker } has attempted to punch { $victim }!

# Status bar. Args: $value (health), $intent (help/harm).
hud-health = Health: { $value }
hud-health-crit = Health: CRITICAL
hud-health-dead = You are dead
flash-blinds = { $name } blinds { $victim } with a flash!
flash-click = *click* *click*
flash-burnt-out = The bulb has burnt out!
flasher-disconnected = You disconnect the flashbulb.
flasher-connected = You connect the flashbulb.
flasher-secured = It is now secured.
flasher-loose = It can now be moved.
monkey-emote = { $name } { $act ->
    [jump] jumps!
    [roll] rolls.
    [tail] waves its tail.
   *[scratch] scratches.
}
microwave-on = The microwave turns on.
microwave-added = You put it in the microwave.
microwave-full = The microwave is full.
microwave-unsuitable = You have no idea what you could cook with that.
arcade-villain = { $title ->
    [automatic] The Automatic
    [farmer] Farmer
    [lord] Lord
    [professor] Professor
    [evil] The Evil
    [dread-king] The Dread King
   *[space] The Space
} { $villain ->
    [melonoid] Melonoid
    [murdertron] Murdertron
    [sorcerer] Sorcerer
    [ruin] Ruin
    [jeff] Jeff
    [ectoplasm] Ectoplasm
    [crushulon] Crushulon
   *[coder] Coder
}
arcade-new = Winners Don't Use Spacedrugs
arcade-new-round = New Round
arcade-attack = You attack for { $value } damage!
arcade-heal = You use { $cost } magic to heal for { $value } damage!
arcade-recharge = You regain { $value } points
arcade-won = { $enemy } has fallen! Rejoice!
arcade-steal = { $enemy } steals { $value } of your power!
arcade-drained = You have been drained! GAME OVER
arcade-enemy-heals = { $enemy } heals for { $value } health!
arcade-enemy-attacks = { $enemy } attacks for { $value } damage!
arcade-crushed = You have been crushed! GAME OVER
arcade-stats = Health: { $hp } | Magic: { $mp } | Enemy Health: { $enemyhp }
arcade-button-attack = Attack
arcade-button-heal = Heal
arcade-button-recharge = Recharge Power
arcade-button-new = New Game
airlock-cycling = The airlock begins to cycle.
scanner-occupied = The scanner is already occupied!
cloning-scanned = Subject successfully scanned.
cloning-already-scanned = Subject already in database.
cloning-no-genes = Error: Unable to locate valid genetic data.
cloning-no-pod = Unable to initiate cloning cycle: no pod nearby.
cloning-busy = Unable to initiate cloning cycle: the pod is in use.
cloning-nobody = Nobody in the database can be cloned right now.
cloning-started = Cloning cycle activated.
cloning-begins = Clone generation process initiated. This will take a moment, please hold.
cloning-done = The pod opens. You are alive again.
brig-title = Door timer
brig-time = Time left: { $minutes }:{ $seconds }
brig-start = Activate timer and close door
brig-stop = Stop timer and open door
brig-flash = Activate flash
brig-flash-charging = Flash charging
brig-denied = Access denied.
gun-fires = { $name } fires { NOUN($item, case: "indef") }!
gun-click = *click*
gun-set-kill = It is now set to kill.
gun-set-stun = It is now set to stun.
shot-hit-laser = { $name } is hit by a laser!
shot-hit-electrode = { $name } is hit by an electrode and goes down!
baton-on = The baton is now on.
baton-off = The baton is now off.
baton-empty = The baton is out of charge.
baton-prod = { $name } prods { $victim } with a stun baton. Luckily it was off.
baton-stun = { $name } stuns { $victim } with a stun baton!
recharger-no-power = The recharger blinks red as you try to insert it.
cuffs-applying = { $name } is trying to put handcuffs on { $victim }!
cuffs-applied = { $name } has handcuffed { $victim }!
cuffs-removing = { $name } is trying to take the handcuffs off { $victim }.
cuffs-removed = { $name } takes the handcuffs off { $victim }.
cuffs-resisting = { $name } attempts to remove the handcuffs!
cuffs-broken-out = { $name } manages to remove the handcuffs!
turret-denied = Access denied.
turret-set-stun = The turrets are on, set to stun.
turret-set-lethal = The turrets are on, set to kill.
turret-set-off = The turrets are off.
syringe-no-blood = You are unable to locate any blood.
syringe-nothing = There is nothing to draw, or the syringe is full.
syringe-drawn = You fill the syringe with { $value } units.
syringe-empty = It is empty.
syringe-injecting = { $name } is trying to inject { $victim }!
syringe-injected = You inject { $value } units.
hypo-injected = You inject { $value } units with the hypospray.
pill-swallowed = You swallow the pill.
pill-fed = You force { $victim } to swallow the pill.
glass-poured = You transfer { $value } units.
glass-nothing = There is nothing to pour, or no room for it.
reagents-contains = It contains { $value } units.
reagents-empty = It is empty.
reagent-aluminium = Aluminium
reagent-carbon = Carbon
reagent-chlorine = Chlorine
reagent-copper = Copper
reagent-ethanol = Ethanol
reagent-fluorine = Fluorine
reagent-hydrogen = Hydrogen
reagent-iron = Iron
reagent-lithium = Lithium
reagent-mercury = Mercury
reagent-nitrogen = Nitrogen
reagent-oxygen = Oxygen
reagent-phosphorus = Phosphorus
reagent-potassium = Potassium
reagent-radium = Radium
reagent-sacid = Sulphuric acid
reagent-silicon = Silicon
reagent-sodium = Sodium
reagent-sugar = Sugar
reagent-sulfur = Sulfur
reagent-water = Water
dispenser-title = Chem dispenser. Energy: { $value }
dispenser-beaker = Beaker: { $value } of { $max } units
dispenser-no-beaker = No beaker loaded.
dispenser-eject = Eject beaker
dispenser-occupied = A beaker is already loaded into the machine.
reagent-anti_toxin = Anti-toxin
reagent-kelotane = Kelotane
reagent-tricordrazine = Tricordrazine
reagent-dexalinp = Dexalin Plus
reagent-synaptizine = Synaptizine
reagent-cryoxadone = Cryoxadone
reagent-toxin = Toxin
reagent-nutriment = Nutriment
reagent-gold = Gold
reagent-silver = Silver
reagent-plasma = Plasma
reagent-inaprovaline = Inaprovaline
reagent-stoxin = Sleep toxin
reagent-dermaline = Dermaline
reagent-bicaridine = Bicaridine
reagent-dexalin = Dexalin
medbay-put-in = { $name } puts { $victim } inside.
medbay-occupied = It is already occupied.
medbay-beaker-full = A beaker is already loaded.
sleeper-numb = You feel cool air surround you. You go numb as your senses turn inward.
sleeper-empty = Sleeper unoccupied.
sleeper-occupant = { $name }: { $state ->
    [conscious] conscious
    [unconscious] unconscious
   *[dead] DEAD
}
sleeper-readings = Health: { $value }. Oxygen damage: { $oxy }. Toxin damage: { $tox }.
sleeper-inject = Inject { $reagent } ({ $value } units in them)
sleeper-unstable = ERROR: Subject is not in stable condition for auto-injection.
sleeper-cant-metabolise = ERROR: Subject cannot metabolise chemicals.
cryo-on = You turn the cryo cell on.
cryo-off = You turn the cryo cell off.
master-title = CheMaster 3000
master-line = { $reagent }: { $value } units
master-eject = Eject beaker and clear buffer
master-to-buffer = Beaker (click to move 10 units to the buffer):
master-to-beaker = Buffer (click to move 10 units back):
master-pill = Create pill (up to 50 units)
master-bottle = Create bottle (up to 30 units)
tank-filled = You fill it with { $value } units.
tank-empty = It is empty.
tank-glass-full = That is already full.
vending-denied = Access denied.
reagent-condensedcapsaicin = Condensed capsaicin
reagent-beer = Beer
reagent-fuel = Welding fuel
gun-loaded = You load { $value } rounds into the gun.
gun-full = It's already full.
gun-wrong-ammo = That doesn't fit this gun.
gun-other-ammo = There's another kind of round loaded. Fire those first.
ammo-empty = There's nothing left in it.
shot-hit-bullet = { $name } is hit by a bullet!
lathe-title = Metal: { $metal } cm³. Glass: { $glass } cm³.
lathe-busy = Working...
lathe-row = { NOUN($item) } ({ $metal } m, { $glass } g)
lathe-fed = You feed { $value } { $value ->
    [one] sheet
   *[other] sheets
} in.
lathe-full = It has no room for that.
lathe-refuses = It doesn't take that.
scanner-empty = The scanner is empty.
scanner-unhurt = No wounds found.
scanner-part = { NOUN($item) }: { $value } brute, { $burn } burn.
job-captain = Captain
job-hop = Head of Personnel
job-hos = Head of Security
job-chief-engineer = Chief Engineer
job-research-director = Research Director
job-security-officer = Security Officer
job-warden = Warden
job-forensic-technician = Forensic Technician
job-engineer = Engineer
job-atmos-technician = Atmospheric Technician
job-medical-doctor = Medical Doctor
job-chemist = Chemist
job-geneticist = Geneticist
job-scientist = Scientist
job-roboticist = Roboticist
job-hydroponicist = Hydroponicist
job-barman = Barman
job-chef = Chef
job-janitor = Janitor
job-quartermaster = Quartermaster
job-cargo-technician = Cargo Technician
job-shaft-miner = Shaft Miner
job-counselor = Counselor
job-clown = Clown
job-mime = Mime
job-assistant = Assistant
job-assigned = You are the { $job ->
    [captain] Captain
    [hop] Head of Personnel
    [hos] Head of Security
    [chief-engineer] Chief Engineer
    [research-director] Research Director
    [security-officer] Security Officer
    [warden] Warden
    [forensic-technician] Forensic Technician
    [engineer] Engineer
    [atmos-technician] Atmospheric Technician
    [medical-doctor] Medical Doctor
    [chemist] Chemist
    [geneticist] Geneticist
    [scientist] Scientist
    [roboticist] Roboticist
    [hydroponicist] Hydroponicist
    [barman] Barman
    [chef] Chef
    [janitor] Janitor
    [quartermaster] Quartermaster
    [cargo-technician] Cargo Technician
    [shaft-miner] Shaft Miner
    [counselor] Counselor
    [clown] Clown
    [mime] Mime
   *[assistant] Assistant
}.
job-window-title = You're down as { $job }. Choose that, or another job that's open.
job-keep = Start in this job
job-row = { $job } ({ $value } free)
records-security = Security records. Click someone to change their status.
records-medical = Medical records
records-empty = There are no records.
records-row = { $name }, { $job }: { $status }
records-denied = Access denied.
record-none = None
record-arrest = *Arrest*
record-incarcerated = Incarcerated
record-parolled = Parolled
record-released = Released
record-active = Active
record-unconscious = *Unconscious*
record-deceased = *Deceased*
id-computer-empty = Put a card in, by clicking with it in your hand.
id-computer-card = Card of { $name }. Job: { $job }.
id-computer-no-job = none
id-computer-eject = Eject the card
examine-card = It belongs to { $name }, { $job ->
    [captain] Captain
    [hop] Head of Personnel
    [hos] Head of Security
    [chief-engineer] Chief Engineer
    [research-director] Research Director
    [security-officer] Security Officer
    [warden] Warden
    [forensic-technician] Forensic Technician
    [engineer] Engineer
    [atmos-technician] Atmospheric Technician
    [medical-doctor] Medical Doctor
    [chemist] Chemist
    [geneticist] Geneticist
    [scientist] Scientist
    [roboticist] Roboticist
    [hydroponicist] Hydroponicist
    [barman] Barman
    [chef] Chef
    [janitor] Janitor
    [quartermaster] Quartermaster
    [cargo-technician] Cargo Technician
    [shaft-miner] Shaft Miner
    [counselor] Counselor
    [clown] Clown
    [mime] Mime
   *[assistant] Assistant
}.
pot-occupied = Something is already growing in it.
pot-planted = You plant the seed.
pot-cleared = You clear away the dead plant.
pot-harvested = You harvest { $value } from the plant.
pot-not-ready = It isn't ready to harvest yet.
examine-pot-empty = Nothing is growing in it. It holds { $value } units of water.
examine-pot-dead = The plant in it is dead.
examine-pot-ripe = The plant in it is ready to harvest. It holds { $value } units of water.
examine-pot-growing = Something is growing in it. It holds { $value } units of water.
barrier-locked = Barrier lock toggled on.
barrier-unlocked = Barrier lock toggled off.
barrier-repaired = You repair the barrier.
barrier-hit = You hit the barrier.
barrier-broken = The barrier falls apart.
apc-hand-full = You need an empty hand to work the APC.
round-mode = The current game mode is { $mode ->
    [traitor] Traitor
    [meteor] Meteor
    [malfunction] AI Malfunction
    [nuclear] Nuclear Emergency
    [revolution] Revolution
    [blob] Blob
    [changeling] Changeling
    [rp-revolution] RP Revolution
    [monkey] Monkey
    [autotraitor] AutoTraitor
    [derelict] Derelict
    [restructuring] Corporate Restructuring
    [sandbox] Sandbox
   *[extended] Extended
}. Enjoy the game!
lobby-title = Welcome to the pre-game lobby!
lobby-countdown = The round starts in { $value } seconds. Ready: { $ready } of { $players }.
lobby-ready = Declare ready
lobby-unready = Undeclare ready
lobby-round-on = The round is under way.
lobby-join = Join the game
lobby-name = Your name: { $name }. Press T to change it.
lobby-no-name = You'll get a random name. Press T to type one.
lobby-prefer = The job you'd like:
comms-title = Communications console
comms-call = Call the escape pods
comms-recall = Stop the launch
comms-launching = The escape pods launch in { $value } { $value ->
    [one] second
   *[other] seconds
}.
comms-gone = The escape pods have gone.
comms-denied = Access denied.
pods-called = Alert: escape pods launching in { $value } { $value ->
    [one] minute
   *[other] minutes
}. Their doors are unlocked.
pods-already-called = Launch process already started!
pods-recalled = Alert: escape pods launch sequence aborted.
pods-minutes = { $value } { $value ->
    [one] minute
   *[other] minutes
} until escape pod launch.
pods-seconds = { $value } { $value ->
    [one] second
   *[other] seconds
} until escape pod launch.
pods-launched = The escape pods have launched!
round-over = The round is over. Restarting in 60 seconds.
round-escaped = { $name }, { $job }, escaped.
round-stranded = { $name }, { $job }, was left behind.
round-died = { $name }, { $job }, died.
id-manifest-open = Access Crew Manifest
id-manifest-title = Crew Manifest:
id-manifest-hint = Please use security record computer to modify entries.
id-manifest-row = { $name } - { $job }
id-manifest-print = Print
id-manifest-printing = Printing...
id-manifest-back = Back
id-manifest-paper = paper- 'Crew Manifest'
traitor-you = You are the traitor.
traitor-repeat = REPEAT
traitor-reminder = In case you missed it the first time - YOU ARE THE TRAITOR!
traitor-uplink-pack = The Syndicate have hidden a ship bounced radio in your backpack. Use it in your hand for the uplink, and to go over your objectives again.
traitor-uplink-hand = The Syndicate have put a ship bounced radio in your hand. Use it there for the uplink, and to go over your objectives again.
traitor-uplink-floor = The Syndicate have left a ship bounced radio at your feet. Use it in your hand for the uplink, and to go over your objectives again.
objective-steal = Objective #{ $value }: Steal the { NOUN($item) }.{ $done ->
    [won] {" "}Success
    [failed] {" "}Failed
   *[none] {""}
}
objective-assassinate = Objective #{ $value }: Assassinate { $name }, the { $job }.{ $done ->
    [won] {" "}Success
    [failed] {" "}Failed
   *[none] {""}
}
objective-protect = Objective #{ $value }: { $name }, the { $job }, is a relative of a high ranking Syndicate leader. Make sure they get off the station safely.{ $done ->
    [won] {" "}Success
    [failed] {" "}Failed
   *[none] {""}
}
objective-escape = Objective #{ $value }: Escape on the pods alive, without being arrested.{ $done ->
    [won] {" "}Success
    [failed] {" "}Failed
   *[none] {""}
}
objective-hijack = Objective #{ $value }: Hijack the escape pods by being the only one to get away.{ $done ->
    [won] {" "}Success
    [failed] {" "}Failed
   *[none] {""}
}
uplink-title = Syndicate Uplink Console:
uplink-crystals = Tele-Crystals left: { $value }
uplink-good = { NOUN($item) } ({ $value })
round-traitor = The syndicate traitor was { $name }.
round-traitor-won = The traitor was successful!
round-traitor-lost = The traitor has failed!
vote-started = A vote to { $kind ->
    [mode] change the game mode
   *[restart] restart
} has been started. You have { $value } seconds. Press V to vote.
vote-title = A vote to { $kind ->
    [mode] change the game mode
   *[restart] restart
} is in progress.
vote-closes-in = { $value } seconds until voting is closed.
vote-next-in = The next vote can begin in { $value } seconds.
vote-leading = Current winner: { $choice }
vote-begin-restart = Begin restart vote
vote-begin-mode = Begin change mode vote
vote-choice-no = No
vote-choice-restart = Yes
vote-choice-default = No change
vote-choice-secret = Secret
vote-choice-extended = Extended
vote-choice-traitor = Traitor
vote-result-no-restart = Voting has closed. Result is no restart.
vote-result-restart = Voting has closed. Result is restart: the server restarts in { $value } seconds.
vote-result-no-change = Voting has closed. Result is no change.
vote-result-mode = Voting has closed. Result is a change to { $mode }.
vote-reboot = The server restarts in { $value } seconds.
lobby-vote = Vote
emote-blink = { $name } blinks.
emote-blink-r = { $name } blinks rapidly.
emote-wave = { $name } waves.
emote-choke = { $name } chokes!
emote-clap = { $name } claps.
emote-drool = { $name } drools.
emote-eyebrow = { $name } raises an eyebrow.
emote-chuckle = { $name } chuckles.
emote-twitch = { $name } twitches violently.
emote-twitch-s = { $name } twitches.
emote-faint = { $name } faints.
emote-cough = { $name } coughs!
emote-frown = { $name } frowns.
emote-nod = { $name } nods.
emote-blush = { $name } blushes.
emote-gasp = { $name } gasps!
emote-giggle = { $name } giggles.
emote-grin = { $name } grins.
emote-cry = { $name } cries.
emote-sigh = { $name } sighs.
emote-laugh = { $name } laughs.
emote-mumble = { $name } mumbles!
emote-grumble = { $name } grumbles!
emote-groan = { $name } groans!
emote-howl = { $name } howls!
emote-moan = { $name } moans!
emote-raise = { $name } raises a hand.
emote-shrug = { $name } shrugs.
emote-smile = { $name } smiles.
emote-shiver = { $name } shivers.
emote-pale = { $name } goes pale for a second.
emote-tremble = { $name } trembles in fear!
emote-sneeze = { $name } sneezes.
emote-sniff = { $name } sniffs.
emote-snore = { $name } snores.
emote-whimper = { $name } whimpers.
emote-wink = { $name } winks.
emote-yawn = { $name } yawns.
emote-collapse = { $name } collapses!
emote-scream = { $name } screams!
emote-point = { $name } points.
emote-me = { $name } { $text }
emote-unknown = There's no such emote.
whisper = { $name } whispers, "{ $text }"
whisper-seen = { $name } whispers something.
ooc-say = OOC: { $name }: { $text }
ooc-unnamed = Someone
chat-prompt-ooc = OOC:
chat-prompt-me = Me:
channel-science = Science
channel-command = Command
channel-medical = Medical
channel-engineering = Engineering
channel-security = Security
channel-syndicate = Syndicate
channel-supply = Supply
protected-head = You have been protected from a hit to the head.
protected-chest = You have been protected from a hit to the chest.
protected-groin = You have been protected from a hit to the groin.
knocked-out = { $victim } has been knocked unconscious!
knocked-down = { $victim } has been knocked down!
stunned = { $victim } has been stunned!
punch-weakened = { $attacker } has weakened { $victim }!
punch-knocked-down = { $attacker } has knocked down { $victim }!
punch-stunned = { $attacker } has stunned { $victim }!
pushed-down = { $attacker } has pushed down { $victim }!
disarmed = { $attacker } has disarmed { $victim }!
disarm-missed = { $attacker } has attempted to disarm { $victim }!
shakes-awake = { $attacker } shakes { $victim }, trying to wake them up!
checks-self = { $name } looks themselves over.
self-part-ok = My { NOUN($zone) } is OK.
self-part-one = My { NOUN($zone) } is { $a }.
self-part-two = My { NOUN($zone) } is { $a } and { $b }.
status-bruised = bruised
status-mangled = mangled
status-numb = numb
status-blistered = blistered
status-peeling = peeling away
grab-passive = { $attacker } has grabbed { $victim } passively!
grab-aggressive = { $attacker } has grabbed { $victim } aggressively (now hands)!
grab-failed = { $attacker } has failed to grab { $victim } aggressively!
grab-neck = { $attacker } has reinforced their grip on { $victim } (now neck)!
grab-choke = { $attacker } has tightened their grip on { $victim }'s neck!
grab-loosen = { $attacker } has loosened the grip on { $victim }'s neck!
grab-broke-free = { $victim } has broken free of { $attacker }'s grip!
grab-broke-headlock = { $victim } has broken free of { $attacker }'s headlock!
grab-on-table = { $attacker } puts { $victim } on the table.
grab-take-off = You have to take off their { NOUN($item) } first!
armor-absorbs = Your armor absorbs the blow!
armor-softens = Your armor softens the blow!
slipped = You slipped on the { NOUN($item) }!
explosion-heard = An explosion rocks the station!
build-title = What to make, of { $value } sheets:
build-recipe = { NOUN($item) } ({ $cost } { $cost ->
    [one] sheet
   *[other] sheets
})
build-recipe-many = { $made } x { NOUN($item) } ({ $cost } { $cost ->
    [one] sheet
   *[other] sheets
})
build-need-more = You don't have enough.
build-no-room = There's no room for it here.
build-too-many-panes = There are too many windows here.
build-need-fuel = You need more welding fuel to do that.
build-unwelding = Now disassembling the outer wall plating.
build-unwelded = You disassembled the outer wall plating.
build-interrupted = You stop what you were doing.
admin-your-key = Your key is { $key }.
admin-not-admin = You aren't an admin.
admin-low-rank = You need a higher rank for that.
admin-unknown = There's no admin verb "{ $verb }". Try help.
admin-help = Verbs: who, asay TEXT, dsay TEXT, pm WHO TEXT, announce TEXT, startnow, toggleooc, togglespawning, jump X Y [Z], jumpto WHO, kick WHO [WHY], ban WHO MINUTES [WHY] (0 for good), unban KEY, bans, restart, mute WHO, explode SIZE, emp HEAVY LIGHT, spell WHO SPELL, toggleevents, event [NAME], get WHO, spawn THING, delete THING, god WHO, rejuv WHO.
admin-welcome = You are an admin. Press F2 for the admin line, then type help.
admin-no-one = There's nobody called { $name }.
admin-no-thing = There's no { $name } here.
admin-asay = ADMIN: { $name }: { $text }
ghost-dsay = [DEAD] Admin { $name }: { $text }
admin-pm-from = Admin PM from { $name }: { $text }
admin-pm-to = PM from { $name } to { $target }: { $text }
admin-announce = Priority announcement: { $text }
admin-started = { $name } has started the round.
admin-ooc-on = OOC has been turned on.
admin-ooc-off = OOC has been turned off.
admin-kicked = You have been kicked by { $name }. { $text }
admin-banned = You have been banned by { $name }. { $text }
admin-unbanned = { $key } is no longer banned.
admin-no-ban = { $key } isn't banned.
admin-ban-row = { $key }, by { $by }, { $value ->
    [-1] for good
   *[other] { $value } minutes left
}: { $text }
admin-muted = You have been muted.
admin-voiced = You have been voiced.
admin-did = { $name } used { $verb } on { $target }.
admin-who-row = { $name } ({ $key }), { $job }, { $address }
admin-help-from = HELP: { $name } ({ $key }): { $text }
admin-help-sent = PM to admins: { $text }
chat-prompt-ahelp = Adminhelp:
chat-prompt-admin = Admin:
comms-red-on = Declare Red Alert
comms-red-off = Cancel Red Alert
comms-display-title = Set Status Display:
comms-display-pods = Escape pod timer
comms-display-blank = Clear
comms-display-message = Message
comms-picture-default = Alert: None
comms-picture-redalert = Alert: Red Alert
comms-picture-lockdown = Alert: Lockdown
comms-picture-biohazard = Alert: Biohazard
chat-prompt-display = Line 1/Line 2:
rwall-step-0 = You cut the support lines.
rwall-step-1 = Removing support lines.
rwall-step-2 = Slicing metal cover.
rwall-step-3 = Prying cover off.
rwall-step-4 = Detaching support rods.
rwall-step-5 = Removing support rods.
rwall-step-6 = Prying outer sheath off.
hair-hair_a = Short Hair
hair-hair_b = Long Hair
hair-hair_longfringe = Long Fringe
hair-hair_vlongfringe = Very Long Fringe
hair-hair_longest = Longest Fringe
hair-hair_test = Ladylike hair
hair-hair_c = Cut Hair
hair-hair_d = Mohawk
hair-hair_e = Balding
hair-hair_f = Wave
hair-hair_bedhead = Bedhead
hair-hair_bedheadv2 = Bedhead 2
hair-hair_spikey = Spikey
hair-hair_dreads = Dreadlocks
hair-hair_afro = Afro
hair-hair_bigafro = Big Afro
hair-hair_braid = Braid
hair-hair_kagami = Kagami
hair-hair_ponytail = Ponytail
hair-facial_watson = Watson
hair-facial_chaplin = Chaplin
hair-facial_selleck = Selleck
hair-facial_neckbeard = Neckbeard
hair-facial_fullbeard = Full Beard
hair-facial_longbeard = Long Beard
hair-facial_vandyke = Van Dyke
hair-facial_elvis = Elvis
hair-facial_abe = Abe
hair-facial_chin = Chinstrap
hair-facial_hip = Hipster
hair-facial_gt = Goatee
hair-facial_hogan = Hogan
colour-black = black
colour-brown = brown
colour-blonde = blonde
colour-red = red
colour-grey = grey
colour-white = white
colour-blue = blue
colour-green = green
hair-bald = bald
hair-shaved = shaved
pref-male = Gender: male
pref-female = Gender: female
pref-hair = Hair: { $what }
pref-hair-colour = Hair colour: { $what }
pref-facial = Face: { $what }
pref-eyes = Eyes: { $what }
pref-darker = Darker skin ({ $value })
pref-lighter = Lighter skin
pref-underwear = Underwear: { $value }
pref-traitor-on = Would be a traitor: yes
pref-traitor-off = Would be a traitor: no
pref-job-slot = Clicking a job sets choice { $value }
lobby-observe = Observe
lobby-crew = View the Crew Manifest
ghost-observing = You are watching as a ghost. The living can't see or hear you.
round-mode-extended = Just have fun and role-play!
round-mode-sandbox = Build your own ship: F5 opens the sandbox panel.
sandbox-title = Sandbox panel
sandbox-suit = Suit up (space travel gear)
sandbox-kit = { NOUN($item) } ({ $value })
sandbox-object = { NOUN($item) }
sandbox-objects = Any item, page { $value } of { $pages }:
sandbox-previous = Previous page
sandbox-next = Next page
sandbox-spawning-on = Sandbox: item spawning has been turned on.
sandbox-spawning-off = Sandbox: item spawning has been turned off.
vote-choice-sandbox = Sandbox
nuke-title = Nuclear Fission Explosive
nuke-disk-in = Auth. disk: ++++++++++
nuke-disk-out = Auth. disk: ----------
nuke-time = Time: { $value }
nuke-on = on
nuke-off = off
nuke-timer = Timer: { $state }
nuke-safety = Safety: { $state }
nuke-anchor = Anchor: { $state }
nuke-display-auth = AUTH
nuke-blast = The station shakes as a blinding light swallows everything!
nuke-win-syndicate = The Syndicate's operatives have destroyed the station!
nuke-killed-everyone = Everyone was killed by the nuclear blast!
nuke-win-crew = The crew escaped with the disk: the Syndicate's operatives have been stopped!
nuke-neutral = Neutral victory: the disk didn't get away, but the station still stands. Next time, don't lose the disk!
nuke-agent = { $name } was a Syndicate agent.
nuke-agent-dead = { $name } was a Syndicate agent. (dead)
nuke-pin-on = You activate the pinpointer. { $deck ->
    [up] The disk is on a deck above.
    [down] The disk is on a deck below.
   *[level] The disk is on this deck.
}
nuke-pin-off = You deactivate the pinpointer.
nuke-agent-brief = You are a Syndicate agent! Take the authentication disk and detonate the nuclear bomb on the station.
nuke-leader-code = The nuclear authorization code is: { $code }
nuke-guide = How to set off the bomb. Bring it to the station first: a click on the folded bomb bolts it down, and it can only be moved again from its window. Then put in the disk, type the code and press E. Turn off the safety, set the time, and start the timer. Taking out the disk now won't stop it. R resets the keypad. Remember: disk, code, safety, timer, disk, run.
nuke-radio = Use ":t" to speak on the encrypted Syndicate radio channel.
vote-choice-nuclear = Nuclear Emergency
pref-nuke-on = Would be a nuclear agent: yes
pref-nuke-off = Would be a nuclear agent: no
rev-not-enough = Not enough players for a revolution: this round is Extended instead.
rev-leader = You are a member of the revolutionaries' leadership!
rev-objective-kill = Objective: kill { $name }, a head of staff.
rev-objective-capture = Objective: capture { $name }, a head of staff, by handcuffing them or knocking them out.
rev-flash-howto = You've been given a flash: flash someone in the face to turn them.
rev-rp-howto = Right-click someone and choose Rev-Convert to ask them to join.
rev-joined = You are now a revolutionary! Help your cause, and don't harm your fellow freedom fighters. Your comrades have a red R over them, your leaders a blue one. Help them bring down the heads!
rev-brainwashed = You have been brainwashed! You are no longer a revolutionary!
rev-remembered = { $name } looks like they just remembered their real allegiance!
rev-already = { $name } is already a revolutionary!
rev-cannot = { $name } can't be a revolutionary!
rev-wait = Wait five seconds before asking them again.
rev-asking = You try to convert { $name }...
rev-accepted = { $name } joins the revolution!
rev-refused = { $name } does not support the revolution!
rev-rejected = You reject this traitorous cause!
rev-question = { $name } asks: do you want to join the revolution?
rev-answer-yes = Yes!
rev-answer-no = No!
rev-reveal = The ship has enemy operatives aboard by the names of { $names }: arrest them at once.
rev-win = The heads of staff were brought down! The revolutionaries win!
rev-lose = The heads of staff managed to stop the revolution!
rev-roll-leaders = The head revolutionaries were:
rev-roll-converted = The converted revolutionaries were:
rev-roll-heads = The heads of staff were:
rev-roll-line = { $name } ({ $state })
rev-dead = dead
rev-alive = survived
verb-rev-convert = Rev-Convert
vote-choice-revolution = Revolution
vote-choice-rp-revolution = RP Revolution
ling-power-absorb = Absorb DNA
ling-power-transform = Transform
ling-power-stasis = Regenerative Stasis
ling-power-hive-channel = Hive Channel
ling-power-hive-absorb = Hive Absorb
ling-power-lesser-form = Lesser Form
ling-power-panacea = Anatomic Panacea
ling-power-deaf-sting = Deaf Sting
ling-power-blind-sting = Blind Sting
ling-power-silence-sting = Silence Sting
ling-power-mimic-voice = Mimic Voice
ling-power-extract-dna = Extract DNA Sting
ling-power-transformation-sting = Transformation Sting
ling-power-paralysis-sting = Paralysis Sting
ling-power-death-sting = Death Sting
ling-power-boost-range = Ranged Sting
ling-power-epinephrine = Epinephrine Sacs
ling-power-chem-synth = Rapid Chemical-Synthesis
ling-power-adv-chem-synth = Advanced Chemical-Synthesis
ling-power-engorged-glands = Engorged Chemical Glands
ling-power-digital-camo = Digital Camouflage
ling-power-rapid-regen = Rapid Regeneration
ling-title = Changeling
ling-stats = Chemicals: { $value }/{ $storage }. Genome points: { $points }. Genomes absorbed: { $absorbed }. Genetic damage: { $damage }.
ling-mimicking = Mimicking: { $name }
ling-give-up = Give up and leave your body
ling-choose = Choose:
ling-cancel = Cancel
ling-evolve = Evolve { $name } ({ $value })
ling-use = { $name } ({ $value })
chat-prompt-mimic = Voice to mimic:
ling-you = You are a changeling!
ling-howto = F6 opens your powers. Grab someone and use Absorb DNA to take their genome. Start a line with :g to speak on the hive channel.
ling-incapacitated = We are incapacitated.
ling-need-chems = We require at least { $value } units of chemicals to do that!
ling-genes = Our genomes are still reassembling. We need time to recover first.
ling-need-grab = We must be grabbing a creature in our active hand to absorb them.
ling-incompatible = This creature is not compatible with our biology.
ling-ruined = This creature's DNA is ruined beyond usability!
ling-busy = We are already absorbing!
ling-absorb-1 = This creature is compatible. We must hold still...
ling-absorb-2 = We extend a proboscis.
ling-absorb-3 = We stab them with the proboscis.
ling-proboscis = { $name } extends a proboscis!
ling-stabs = { $name } stabs { $victim } with the proboscis!
ling-stab-pain = You feel a sharp stabbing pain!
ling-interrupted = Our absorption has been interrupted!
ling-absorbed = We have absorbed { $name }!
ling-you-absorbed = You have been absorbed by the changeling!
ling-sucks = { $name } sucks the fluids from { $victim }!
ling-transforms = Someone transforms into { $name }!
ling-genes-cry = Our genes cry out!
ling-stasis = We will attempt to regenerate our form.
ling-stasis-failed = We lack the chemicals to regenerate.
ling-regenerated = We have regenerated.
ling-wakes = { $name } appears to wake from the dead, having healed all wounds.
ling-hive-has = The airwaves already have that DNA.
ling-hive-sent = We channel the DNA of { $name } to the air.
ling-hive-none = There's no new DNA to absorb from the air.
ling-hive-got = We absorb the DNA of { $name } from the air.
ling-hive-say = { $name }: { $text }
ling-cleansed = We cleanse impurities from our form.
ling-range = Our throat adjusts to launch the sting.
ling-arise = We arise.
ling-camo-on = We distort our form to prevent AI tracking.
ling-camo-off = We return to normal.
ling-voice = We shape our glands to take the voice of { $name }. This will use up chemicals until we stop.
ling-voice-back = We return our vocal glands to their original location.
ling-stung = We stealthily sting { $name }.
ling-prick = You feel a tiny prick.
ling-ears-pop = Your ears pop and begin ringing loudly!
ling-eyes-burn = Your eyes burn horrifically!
ling-muscles = Your muscles begin to painfully tighten.
ling-chest = You feel a tiny prick and your chest becomes tight...
ling-sting-useless = Our sting appears ineffective against its DNA.
ling-was = The changeling was { $name }.
ling-genomes = Genomes absorbed: { $value }
ling-won = The changeling was successful!
ling-lost = The changeling has failed!
objective-absorb = Objective #{ $value }: extract { $goal } compatible genomes. { $done ->
    [won] Success
    [failed] Failed
   *[none] { "" }
}
objective-survive = Objective #{ $value }: stay alive until the end. { $done ->
    [won] Success
    [failed] Failed
   *[none] { "" }
}
vote-choice-changeling = Changeling
round-mode-meteor = The ship has been stuck in a major meteor shower. You must escape from the ship or at least live!
blob-alert = Cent. Com. update: biohazard alert. A level 5 organism may have reached the station. Find it and contain it, and don't leave the quarantine area.
blob-confirmed = Confirmed outbreak of a level 5 biohazard aboard. All personnel must contain the outbreak.
blob-directive = Uncontrolled spread of the biohazard on board. Cent. Com. has issued the directive to destroy the station.
blob-staff-won = The staff has won! The biohazard has been contained.
blob-staff-lost = The staff has lost! The station was destroyed by Cent. Com.
blob-attacks-you = The blob attacks you!
blob-weakened = The blob has weakened { $victim }!
blob-knocked-down = The blob has knocked down { $victim }!
blob-stunned = The blob has stunned { $victim }!
vote-choice-meteor = Meteor
vote-choice-blob = Blob
shields-raised = You raise the station's shields.
shields-lowered = You lower the station's shields.
shields-no-charge = Nothing happens: the shield capacitors hold too little charge.
shield-converter-mode = You set the converter { $mode ->
    [manual] to manual
    [auto] to automatic
   *[off] off
}.
shield-capacitor-charge = It's { $value }% charged.
hud-health-ghost = You are a ghost. Press R to start over as a new character
ghost-left-body = You leave your body behind. The living can't see or hear you now.
respawn-no-room = Every start point is taken. Try again in a moment.
ghost-say = [DEAD] { $name } { $verb ->
    [ask] asks
    [exclaim] exclaims
   *[say] says
}, "{ $text }"
hud-intent = Intent: { $intent ->
    [disarm] disarm
    [grab] grab
        [harm] harm
       *[help] help
    }

# Environment. Args: $value (kPa).
gasp = You gasp for air!
hud-pressure = Pressure: { $value } kPa
alert-vacuum = VACUUM
alert-low-pressure = Low pressure
alert-high-pressure = High pressure
alert-low-oxygen = Not enough oxygen
alert-cold = You are freezing
alert-hot = You are overheating

# Machinery. Args: $item (noun id), $value (kPa).
switched-on = You switch the { NOUN($item) } on.
switched-off = You switch the { NOUN($item) } off.
valve-opened = You open the { NOUN($item) }.
valve-closed = You close the { NOUN($item) }.
canister-connected = You connect the { NOUN($item) } to the port.
canister-disconnected = You disconnect the { NOUN($item) } from the port.
examine-gauge = The gauge reads { $value } kPa.

# Power. Args: $value (percent); $equipment, $lighting, $environment (on/off); $item (noun id).
examine-apc = The APC's cell is at { $value }%. Equipment: { $equipment ->
        [on] on
       *[off] off
    }, lighting: { $lighting ->
        [on] on
       *[off] off
    }, environment: { $environment ->
        [on] on
       *[off] off
    }.
examine-smes = The SMES is { $value }% charged.
apc-locked = Access denied.
door-forced = You force the { NOUN($item) } with the crowbar.

# Equipment. Args: $item (noun id).
equip-wrong-slot = You can't wear that there.
equip-occupied = You are already wearing something there.
equip-needs-uniform = You need a jumpsuit for that.
internals-on = You are now running on internals from the { NOUN($item) }.
internals-off = You are no longer running on internals.
internals-no-mask = You are not wearing a breath mask.
internals-no-tank = You have no gas tank to connect to.

# Inventory bar.
hud-slot = { $slot ->
        [uniform] Uniform
        [suit] Suit
        [head] Head
        [mask] Mask
        [gloves] Gloves
        [shoes] Shoes
        [back] Back
        [belt] Belt
        [glasses] Glasses
        [ears] Ears
       *[id] ID
    }

# HUD hover tips (Luna's screen object names). Args: $what.
hud-tip = { $what ->
        [hand-left] Left hand
        [hand-right] Right hand
        [swap] Swap hands
        [drop] Drop
        [pull] Stop pulling
        [zone] Damage zone
        [intent] Intent
        [move] Run / walk
        [internals] Internals
        [oxygen] Oxygen
        [toxin] Toxins
        [fire] Fire
        [temperature] Body temperature
       *[health] Health
    }

# Airlock maintenance panel and wires. Args: $item (noun id), $name, $colour.
door-panel-opened = You open the maintenance panel of the { NOUN($item) }.
door-panel-closed = You close the maintenance panel of the { NOUN($item) }.
wire-need-cutters = You need wirecutters!
wire-need-multitool = You need a multitool!
wire-pulse-cut = You can't pulse a cut wire.
shocked = You feel a powerful shock course through your body!
shocked-seen = { $name } was shocked!
panel-title = Access panel
panel-wire = { $colour ->
        [orange] Orange
        [dark-red] Dark red
        [white] White
        [yellow] Yellow
        [red] Red
        [blue] Blue
        [green] Green
        [grey] Grey
        [black] Black
        [pink] Pink
       *[brown] Brown
    } wire:
panel-cut = Cut
panel-mend = Mend
panel-pulse = Pulse
panel-close = Close
panel-bolts-down = The door bolts have dropped!
panel-bolts-up = The door bolts are up.
panel-test-light-on = The test light is on.
panel-test-light-off = The test light is off!
panel-ai-light-on = The "AI control allowed" light is on.
panel-ai-light-off = The "AI control allowed" light is off.
panel-safety-flashing = The safety light is flashing!
panel-safety-on = The safety light is on.
panel-hydraulics-green = The hydraulics light is solid green.
panel-hydraulics-red = The hydraulics light is flashing red.

# Air alarm. Args: $item (noun id), $name, $area (formatted area name), $value.
alarm-deactivated = { $name } has deactivated the { NOUN($item) }!
alarm-reactivated = { $name } has reactivated the { NOUN($item) }!
alarm-title = { $area } atmosphere
alarm-pressure = Pressure: { $value } kPa
alarm-nitrogen = Nitrogen: { $value }%
alarm-oxygen = Oxygen: { $value }%
alarm-co2 = CO₂: { $value }%
alarm-plasma = Plasma: { $value }%
alarm-unknown = Unknown: { $value }%
alarm-temperature = Temperature: { $value } °C
alarm-seal-on = Activate emergency seal
alarm-seal-off = Deactivate emergency seal
alarm-panic-on = Activate panic fill
alarm-panic-off = Deactivate panic fill
alarm-no-vents = No vents connected!

# Area names.
area-engineering = Engineering
area-eng-storage = Engineering storage
# NSV Luna's engineering section (Luna's area names).
area-engine-hallway = Engine Hallway
area-engine-control = Engine Control
area-engine-monitoring = Engine Monitoring
area-engine-combustion = Engine Combustion Chamber
area-engine-walls = Engine Walls
area-electrical-storage = Electrical Storage
area-chief-engineer = Chief Engineer's Office
area-maint-fore-starboard = Engineering Deck Fore Starboard Maintenance
area-maint-starboard = Sub Deck Starboard Maintenance
area-maint-port = Engineering Deck Port Maintenance
area-maint-port-central = Engineering Port Central Maintenance
area-shield-generation = Shield Generation
area-escape-hangar = Crew Escape Hangar

# Containers. Args: $name, $item and $target (noun ids).
storage-added = { $name } has added { NOUN($item) } to { NOUN($target) }!
storage-cannot-hold = The { NOUN($target) } can't hold { NOUN($item) }.
hud-storage-close = Close

# Cables and cable coils. Args: $name (who), $item (noun id), $count (lengths of cable).
cable-too-far = You can't lay cable at a place that far away.
cable-floor-intact = You can't lay cable there unless the floor tiles are removed.
cable-exists = There's already a cable at that position.
cable-cut = { $name } disconnects the { NOUN($item) }.
coil-cut-piece = You cut a piece off the cable coil.
coil-joined = You join the cable coils together.
coil-transferred = You transfer { $count } { $count ->
        [one] length
       *[other] lengths
    } of cable from one coil to the other.
coil-full = The coil is too long, you cannot add any more cable to it.
examine-coil-short = A short piece of power cable.
examine-coil-piece = A piece of power cable.
examine-coil = A coil of power cable. There are { $count } lengths of cable in the coil.
table-disassembling = Now disassembling table.
table-disassembling-reinforced = Now disassembling the reinforced table.
table-weakening = Now weakening the reinforced table.
table-weakened = Table weakened.
table-strengthening = Now strengthening the reinforced table.
table-strengthened = Table strengthened.
meter-reading = Pressure: { $value } kPa
meter-no-pipe = Results: Connection Error!
meter-too-far = You are too far away.
examine-meter = A gas flow meter. The pressure gauge reads { $value } kPa.
examine-meter-no-pipe = A gas flow meter. The connect error light is blinking.
welder-on = You will now weld when you attack.
welder-off = Not welding anymore.
welder-no-fuel = Need more fuel!
welder-short = Need more welding fuel!
welder-refueled = Welder refueled.
examine-welder = It contains { $count } { $count ->
        [one] unit
       *[other] units
    } of fuel left!
examine-fuel-tank = It contains { $count } { $count ->
        [one] unit
       *[other] units
    } of welding fuel.
examine-floor-tiles = There { $count ->
        [one] is { $count } floor tile
       *[other] are { $count } floor tiles
    } in the stack.
lattice-built = Constructing support lattice ...
lattice-needs-support = The plating is going to need some support.
lattice-sliced = Slicing lattice joints ...

# Lights (docs/reference/lights.md).
light-no-fitting = There is no { NOUN($item) } in this light.
light-removed = You remove the { NOUN($item) }.
light-burn-hand = You try to remove the { NOUN($item) }, but burn your hand on it!
light-smash-burn = You smash the { NOUN($item) }, but burn your hand on it!
light-inserted = You insert the { NOUN($item) }.
light-replaced = You replace the { NOUN($item) }.
light-wrong-type = This type of light requires { NOUN($item, case: "indef") }.
light-stick = You stick the { NOUN($item) } into the light socket!
light-hit = You hit the light!
light-smashed = You hit the light, and it smashes!
light-smashed-seen = { $name } smashed the light!
examine-light-on = It is turned on.
examine-light-off = It is turned off.
examine-light-empty = The { NOUN($item) } has been removed.
examine-light-burned = The { NOUN($item) } is burnt out.
examine-light-broken = The { NOUN($item) } has been smashed.

# Closets, lockers and crates (docs/reference/closets.md).
closet-wont-budge = It won't budge!
closet-welded = The { NOUN($item) } has been welded shut by { $name }.
closet-unwelded = The { NOUN($item) } has been unwelded by { $name }.
closet-cut-apart = The { NOUN($item) } has been cut apart by { $name } with the welding tool.
locker-locked = The locker has been locked by { $name }.
locker-unlocked = The locker has been unlocked by { $name }.
locker-access-denied = Access Denied
locker-locked-inside = The locker is locked!

# Medical (medical.md). Args: $name (who treats), $target, $item (noun id), numbers.
medical-applied = { $target } has been applied with { NOUN($item) } by { $name }
medical-applied-self = { $name } applied { NOUN($item) } on themselves
analyzer-used = { $name } has analyzed { $target }'s vitals.
analyzer-used-self = You have analyzed { $target }'s vitals.
analyzer-status = Analyzing Results for { $target }: Overall Status: { $dead ->
        [yes] dead
       *[no] { $health }% healthy
    }
analyzer-damage = Key: Suffocation/Toxin/Burns/Brute. Damage Specifics: { $oxy } - { $tox } - { $burn } - { $brute }
analyzer-temperature = Body Temperature: { $celsius }°C ({ $fahrenheit }°F)
analyzer-localized = Localized Damage, Brute/Burn:
analyzer-organ = { NOUN($zone) }: { $brute } - { $burn }
analyzer-limbs-ok = Limbs are OK.
analyzer-summary = { $oxy ->
        [severe] Severe oxygen deprivation detected
       *[ok] Subject bloodstream oxygen level normal
    } | { $tox ->
        [severe] Dangerous amount of toxins detected
       *[ok] Subject bloodstream toxin level minimal
    } | { $burn ->
        [severe] Severe burn damage detected
       *[ok] Subject burn injury status O.K
    } | { $brute ->
        [severe] Severe anatomical damage detected
       *[ok] Subject brute-force injury status O.K
    }

# Fire alarm (fire-alarm.md). Args: $name, $item (noun id).
fire-alarm-raised = You pull the fire alarm.
fire-alarm-reset = You reset the fire alarm.
fire-alarm-disconnected = { $name } has disconnected the { NOUN($item) }'s detecting unit!
fire-alarm-reconnected = { $name } has reconnected the { NOUN($item) }'s detecting unit!

# Portable generator (generators.md). Args: $count, $watts, $on (yes/no).
pacman-fuel-added = You add { $count } { $count ->
    [one] sheet
   *[other] sheets
} of plasma to the generator.
pacman-full = The generator is full!
pacman-secured = You secure the generator to the floor.
pacman-unsecured = You unsecure the generator from the floor.
pacman-on = You switch the generator on.
pacman-off = You switch the generator off.
pacman-no-fuel = The generator has no fuel.
examine-pacman = The generator is { $on ->
    [yes] on
   *[no] off
}. It has { $count } { $count ->
    [one] sheet
   *[other] sheets
} of plasma left, producing { $watts } W.

# Tank storage unit (tank-dispenser.md). Args: $item (noun id), $count.
dispenser-taken = You take the { NOUN($item) } out of the tank storage unit.
dispenser-put = You put the { NOUN($item) } in the tank storage unit.
dispenser-full = The tank storage unit is full.
dispenser-empty = The tank storage unit is empty.
examine-tank-dispenser = It holds { $count } { $count ->
    [one] oxygen tank
   *[other] oxygen tanks
}.

# Power cells and cell chargers (cells.md). Args: $name, $rating, $value, $charge (-1: none).
charger-inserted-self = You insert a cell into the charger.
charger-inserted = { $name } inserts a cell into the charger.
charger-removed-self = You remove the cell from the charger.
charger-removed = { $name } removes the cell from the charger.
charger-occupied = There is already a cell in the charger.
charger-blinks = The cell charger blinks red as you try to insert the cell!
charger-remove-cell-first = Remove the cell first!
charger-attached = You attach the cell charger to the ground.
charger-detached = You detach the cell charger from the ground.
examine-cell = The manufacturer's label states this cell has a power rating of { $rating }. The charge meter reads { $value }%.
examine-cell-charger = { $charge ->
    [-1] There's no cell in the charger.
   *[other] There's a cell in the charger. Current charge: { $charge }.
}

# Jetpacks (jetpacks.md).
jetpack-on = You toggle the jetpack on.
jetpack-off = You toggle the jetpack off.

# Radio (radio.md). Args: $name, $frequency ("145.9"), $verb (say/ask/exclaim), $text.
radio-say = [{ $frequency }] { $name } { $verb ->
        [ask] asks
        [exclaim] exclaims
       *[say] says
    }, "{ $text }"
radio-microphone-on = You engage the microphone.
radio-microphone-off = You disengage the microphone.

# Chairs (chairs.md). Args: $name, $target (who was in it).
chair-buckled-self = You buckle yourself to the { NOUN($seat) }.
chair-buckles = { $name } buckles in!
chair-unbuckled-self = You unbuckle yourself from the { NOUN($seat) }.
chair-unbuckles = { $name } unbuckles.
chair-unbuckled-by = { $target } was unbuckled by { $name }!

# APC (power.md): clicks.
apc-close-cover-first = You must close the cover to swipe an ID card.
apc-interface-locked = You lock the APC interface.
apc-interface-unlocked = You unlock the APC interface.
apc-cover-opened = You open the APC cover.
apc-cover-closed = You close the APC cover.
apc-cover-locked = The cover is locked and cannot be opened.
apc-has-cell = There is a power cell already installed.
apc-cell-inserted = You insert the power cell.
apc-cell-removed = You remove the power cell.
# APC window. Args: $area, $on (yes/no), $state, $value, $auto (yes/no), $load (W),
# $mode (0 off, 1 auto-off, 2 on, 3 auto-on).
apc-title = Area Power Controller ({ $area })
apc-hint-locked = (Swipe ID card to unlock interface.)
apc-hint-unlocked = (Swipe ID card to lock interface.)
apc-breaker = Main breaker: { $on ->
    [yes] On
   *[no] Off
}
apc-external = External power: { $state ->
    [2] Good
    [1] Low
   *[0] None
}
apc-cell = Power cell: { $value }% ({ $state ->
    [1] Charging
    [2] Fully charged
   *[0] Not charging
}) ({ $auto ->
    [yes] Auto
   *[no] Off
})
apc-no-cell = Power cell: Not connected.
apc-equipment = Equipment: { $load } W: { $mode ->
    [1] Off (Auto)
    [2] On
    [3] On (Auto)
   *[0] Off
}
apc-lighting = Lighting: { $load } W: { $mode ->
    [1] Off (Auto)
    [2] On
    [3] On (Auto)
   *[0] Off
}
apc-environment = Environmental: { $load } W: { $mode ->
    [1] Off (Auto)
    [2] On
    [3] On (Auto)
   *[0] Off
}
apc-cover-lock = Cover lock: { $on ->
    [yes] Engaged
   *[no] Disengaged
}
apc-toggle = Toggle
apc-set-off = Off
apc-set-on = On
apc-set-auto = Auto

# Power monitoring computer (power.md). Args: $supply, $load (W); $area; $equipment,
# $lighting, $environment (0 off, 1 auto-off, 2 on, 3 auto-on); $value (%); $state
# (0 not charging, 1 charging, 2 full).
monitor-title = Power Monitoring
monitor-totals = Total power: { $supply } W. Total load: { $load } W.
monitor-apc = { $area }: { $equipment ->
    [1] AOff
    [2] On
    [3] AOn
   *[0] Off
} / { $lighting ->
    [1] AOff
    [2] On
    [3] AOn
   *[0] Off
} / { $environment ->
    [1] AOff
    [2] On
    [3] AOn
   *[0] Off
}, { $load } W, cell { $value }% { $state ->
    [1] C
    [2] F
   *[0] N
}
monitor-apc-no-cell = { $area }: { $equipment ->
    [1] AOff
    [2] On
    [3] AOn
   *[0] Off
} / { $lighting ->
    [1] AOff
    [2] On
    [3] AOn
   *[0] Off
} / { $environment ->
    [1] AOff
    [2] On
    [3] AOn
   *[0] Off
}, { $load } W, no cell

# Thermo-electric generator (engine.md). Args: $watts; $hot, $cold (K); $hotkpa, $coldkpa.
examine-teg = Output: { $watts } W. Hot loop: { $hot } K, { $hotkpa } kPa. Cold loop: { $cold } K, { $coldkpa } kPa.

# Engine core (engine.md). Laser computer window; $on (yes/no). Core warnings on the
# radio; $frequency ("145.9").
laser-title = Laser status monitor
laser-none = No laser found
laser-power = Power level:
laser-output = Output: { $on ->
    [yes] Online
   *[no] Offline
}
core-overload = [{ $frequency }] Core control computer: CORE OVERLOAD
core-explosion-imminent = [{ $frequency }] Core control computer: CORE EXPLOSION IMMINENT
core-safe = [{ $frequency }] Core control computer: Core returning to safe operating levels.

# Freezer window (engine.md). Args: $on (yes/no), $temperature (K), $pressure (kPa).
freezer-status = Current status: { $on ->
    [yes] On
   *[no] Off
}
freezer-gas = Gas: { $temperature } K, { $pressure } kPa
freezer-setting = Target gas temperature:

# SMES window (power.md). Args: $value (%), $charging, $auto, $on (yes/no), $load (W).
smes-title = SMES Power Storage Unit
smes-charge = Stored capacity: { $value }%
smes-input = Input: { $charging ->
    [yes] Charging
   *[no] Not charging
} ({ $auto ->
    [yes] Auto
   *[no] Off
})
smes-input-level = Input level:
smes-output = Output: { $on ->
    [yes] Online
   *[no] Offline
}
smes-output-level = Output level:
smes-load = Output load: { $load } W

# Wall-mounted igniter (igniters.md). Args: $name, $item (noun id).
sparker-disabled = { $name } has disabled the { NOUN($item) }!
sparker-disabled-self = You disable the connection to the { NOUN($item) }.
sparker-reconnected = { $name } has reconnected the { NOUN($item) }!
sparker-reconnected-self = You fix the connection to the { NOUN($item) }.

# Emergency floodlight (floodlights.md).
floodlight-on = You turn on the light.
floodlight-off = You turn off the light.
floodlight-cell-removed = You remove the power cell.
floodlight-unscrewed = You unscrew the battery panel.
floodlight-screwed = You screw the battery panel in place.
floodlight-panel-removed = You remove the battery panel.
floodlight-panel-replaced = You crowbar the battery panel in place.
floodlight-cell-already = There is a power cell already installed.
floodlight-cell-inserted = You insert the power cell.

# Disposal unit (disposals.md). Args: $name, $item, $bin (noun ids).
disposal-placed-self = You place the { NOUN($item) } into the { NOUN($bin) }.
disposal-placed = { $name } places the { NOUN($item) } into the { NOUN($bin) }.

# Fire extinguishers (extinguishers.md). Args: $count; $item, $cabinet (noun ids).
extinguisher-safety-on = The safety is on.
extinguisher-safety-off = The safety is off.
extinguisher-empty = The fire extinguisher is empty.
extinguisher-refilled = The fire extinguisher is now refilled.
extinguisher-taken = You take the { NOUN($item) } from the { NOUN($cabinet) }.
extinguisher-placed = You place the { NOUN($item) } in the { NOUN($cabinet) }.
examine-extinguisher = It contains { $count } { $count ->
        [one] unit
       *[other] units
    } of water left!
examine-water-tank = It contains { $count } { $count ->
        [one] unit
       *[other] units
    } of water.

# Right-click menu. Args: $item (noun id).
menu-thing = { NOUN($item) }
menu-examine = Examine
menu-use = Use
menu-pull = Pull

# Emergency wall lockers (walllockers.md). Args: $first, $second, $third, $item (noun ids).
walllocker-empty = It's empty...
walllocker-dispensed-1 = Dispensed a { NOUN($first) }.
walllocker-dispensed-2 = Dispensed a { NOUN($first) } and { NOUN($second) }.
walllocker-dispensed-3 = Dispensed a { NOUN($first) }, { NOUN($second) } and { NOUN($third) }.
walllocker-put-back = You put the { NOUN($item) } back in the locker.

# Canister, portable pump and scrubber windows. Args: $value, $connected, $on, $out (yes/no).
gas-title-canister = Canister
gas-title-pump = Portable Air Pump
gas-title-scrubber = Portable Air Scrubber
gas-pressure = Pressure: { $value } kPa
gas-port = Port status: { $connected ->
        [yes] Connected
       *[no] Disconnected
    }
gas-tank = Tank pressure: { $value } kPa
gas-remove-tank = Remove tank
gas-valve = Release valve: { $on ->
        [yes] Open
       *[no] Closed
    }
gas-power = Power switch: { $on ->
        [yes] On
       *[no] Off
    }
gas-direction = Pump direction: { $out ->
        [yes] Out
       *[no] In
    }
gas-release = Release pressure:
gas-target = Target pressure:
gas-rate = Rate:

# Vending machine window (vending.md).
vending-select = Select an item:
vending-vend = Vend
vending-sold-out = Sold Out

# Eating and drinking (food.md). Args: $item, $tool (noun ids); $verb (bite, chew, nibble,
# gnaw, gobble, chomp); $name, $target (people); $count; $level (empty, almost-empty, half,
# almost-full, full).
food-eat-gobble = You hungrily { food-verb } some of the { NOUN($item) } and gobble it down!
food-eat-begin = You hungrily begin to { food-verb-inf } the { NOUN($item) }.
food-eat = You { food-verb } the { NOUN($item) }.
food-eat-unwilling = You unwillingly { food-verb } a bit of the { NOUN($item) }.
food-eat-full = You cannot force any more of the { NOUN($item) } to go down your throat.
food-verb = { $verb ->
        [chew] chew
        [nibble] nibble
        [gnaw] gnaw
        [gobble] gobble
        [chomp] chomp
       *[bite] bite
    }
# English has one form for both.
food-verb-inf = { food-verb }
drink-gulp = You swallow a gulp of the { NOUN($item) }.
drink-empty = None of the { NOUN($item) } left, oh no!
drink-feeds = { $name } feeds { $target } the { NOUN($item) }.
examine-bitten-once = The { NOUN($item) } was bitten by someone!
examine-bitten = The { NOUN($item) } was bitten { $count } times!
examine-bitten-many = The { NOUN($item) } was bitten multiple times!
examine-drink = The { NOUN($item) } is { $level ->
        [empty] empty!
        [almost-empty] almost empty!
        [half] half full!
        [almost-full] almost full!
       *[full] full!
    }
hunger-1 = You feel hungry
hunger-2 = You feel thirsty
hunger-3 = Perhaps you should grab a bite to eat
hunger-4 = Your stomach rumbles
hunger-5 = Perhaps you should have a drink...
hunger-6 = You feel empty inside
hunger-7 = You feel a bit peckish
hunger-8 = Whatever you last ate didn't do much to fill you up...
hunger-9 = Hmm, some pizza would be nice

# Smoking (smoking.md).
cigarettes-out = You're out of cigs, shit! How you gonna get through the rest of the day...
examine-cigarettes = There { $count ->
        [one] is { $count } cig
       *[other] are { $count } cigs
    } left!
lighter-on = After a few attempts, { $name } manages to light the { NOUN($item) }.
lighter-on-burnt = After a few attempts, { $name } manages to light the { NOUN($item) }, they however burn their finger in the process.
lighter-burn-self = You burn yourself while lighting the lighter.
lighter-off = { $name } quietly shuts off the { NOUN($item) }.
cigarette-lit-welder = { $name } casually lights the { NOUN($item) } with the { NOUN($tool) }, what a badass.
cigarette-lit-lighter = After some fiddling, { $name } manages to light their { NOUN($item) } with the { NOUN($tool) }.
cigarette-lit = { $name } lights their { NOUN($item) } with their { NOUN($tool) }.
cigarette-out = Your { NOUN($item) } goes out.
cigarette-trodden = { $name } calmly drops and treads on the lit { NOUN($item) }, putting it out instantly.

# Object verbs in the right-click menu (verbs.rs).
verb-toggle-open = Toggle Open
verb-toggle-lock = Toggle Lock
verb-rotate-chair = Rotate Chair
verb-toggle-light = Toggle Light
verb-toggle-jetpack = Toggle Jetpack

# Security cameras (cameras.md). Args: $name; $item (noun id).
camera-deactivated = { $name } has deactivated the { NOUN($item) }!
camera-reactivated = { $name } has reactivated the { NOUN($item) }!
camera-static = The screen bursts into static.
cameras-title = Security Cameras
cameras-select = Which camera should you change to?
cameras-view = View
cameras-cancel = Cancel
cameras-deactivated = { " " }(Deactivated)

# Paperwork (paperwork.md). Args: $item (noun id); $count.
paper-bin-take = You take the { NOUN($item) } out of the paper bin.
paper-bin-put = You put the { NOUN($item) } in the paper bin.
paper-bin-empty = The paper bin is empty!
examine-paper-bin = { $count ->
        [0] There are no papers in the bin.
        [one] There is one paper in the bin.
       *[other] There are { $count } papers in the bin.
    }
clipboard-clipped = You clip the paper onto the { NOUN($item) }.
clipboard-pen = You slot the { NOUN($item) } into the clipboard.
clipboard-top = You move the { NOUN($item) } to the top.
clipboard-title = Clipboard
clipboard-remove-pen = Remove Pen
clipboard-add-pen = Add Pen
clipboard-write = Write
clipboard-remove = Remove
clipboard-top-button = Move to top
paper-write-prompt = What text do you wish to add?
paper-write-keys = (Enter: new line. Ctrl+Enter: save. Esc: cancel.)
paper-label-prompt = What would you like to label the paper?
paper-save = Save

# Filing cabinets (filing-cabinets.md). Args: $cabinet, $item (noun ids).
cabinet-put = You put the { NOUN($item) } in the { NOUN($cabinet) }.
cabinet-wont-fit = You can't put the { NOUN($item) } in the { NOUN($cabinet) }!
cabinet-wrenched = You wrench the { NOUN($cabinet) }.
cabinet-unwrenched = You unwrench the { NOUN($cabinet) }.
cabinet-empty = The { NOUN($cabinet) } is empty.
pick-list-title = { NOUN($item) }

# Bedsheets (bedsheets.md). Args: $item (noun id); $count.
linen-bin-put = You put the { NOUN($item) } in the linen bin.
linen-bin-hide = You hide the { NOUN($item) } among the sheets.
linen-bin-take = You take the { NOUN($item) } out of the linen bin.
linen-bin-falls-out = The { NOUN($item) } falls out of the bedsheet!
examine-linen-bin = { $count ->
        [0] There are no bed sheets in the bin.
        [one] There is one bed sheet in the bin.
       *[other] There are { $count } bed sheets in the bin.
    }

# Space heaters (space-heaters.md). Args: $name; $on, $open (yes, no); $cell (yes, no,
# meter); $value.
heater-hatch-closed = The hatch must be open to insert a power cell.
heater-has-cell = There is already a power cell inside.
heater-cell-in = You insert the power cell into the space heater.
heater-cell-in-seen = { $name } inserts a power cell into the space heater.
heater-cell-out = You remove the power cell from the space heater.
heater-cell-out-seen = { $name } removes the power cell from the space heater.
heater-hatch-opened = You open the hatch on the space heater.
heater-hatch-opened-seen = { $name } opens the hatch on the space heater.
heater-hatch-shut = You close the hatch on the space heater.
heater-hatch-shut-seen = { $name } closes the hatch on the space heater.
heater-switched-on = You switch on the space heater.
heater-switched-on-seen = { $name } switches on the space heater.
heater-switched-off = You switch off the space heater.
heater-switched-off-seen = { $name } switches off the space heater.
examine-heater = The heater is { $on ->
        [yes] on
       *[no] off
    } and the hatch is { $open ->
        [yes] open
       *[no] closed
    }. { $cell ->
        [yes] The power cell is installed.
        [no] The power cell is missing.
       *[meter] The charge meter reads { $value }%.
    }
heater-title = Space Heater Control Panel
heater-cell = Power cell:
heater-cell-installed = Installed
heater-cell-removed = Removed
heater-level = Power Level: { $value }%
heater-set = Set Temperature:
heater-degrees = { $value }°C

# Morgue (morgue.md). Args: $name, $victim (names); $item (noun id).
morgue-stuffed = { $name } stuffs { $victim } into the { NOUN($item) }!
crematorium-locked = It's locked.
crematorium-denied = Access denied.
crematorium-crackle = You hear a hollow crackle.
crematorium-roar = You hear a roar as the crematorium activates.
monkey-you-are = You are now a monkey.
monkey-won = The monkeys have won!
monkey-lost = The Research Staff has stopped the monkey invasion!
monkey-was = { $name } was a monkey.
bitten = { $attacker } has bit { $victim }!
bite-missed = { $attacker } has attempted to bite { $victim }!
restructuring-announce = GLOBAL TRANSMISSION FROM HEAD OFFICE: A CORPORATE RESTRUCTURING IS TO TAKE PLACE
restructuring-announce-care = Stay tuned for further news; note that we do care for our employees and any layoffs will be dealt with discretion and compassion
restructuring-target = HEAD OFFICE: { $name } is accused of attempting to start a Union and is now considered a threat to the ship. Terminate the employee immediately.
restructuring-target-job = HEAD OFFICE: { $name } the { $job } is accused of attempting to start a Union and is now considered a threat to the ship. Terminate the employee immediately.
restructuring-you = Head office has ordered your downsizing. Watch out!
vote-choice-monkey = Monkey
vote-choice-restructuring = Corporate Restructuring
autotraitor-announce = This is a test bed for theories and methods to implement an infinite traitor round. Traitors will be added to the round automagically as needed. Expect bugs.
traitor-debt = ATTENTION: It is time to pay your debt to the Syndicate...
traitor-now = You are now a traitor.
vote-choice-autotraitor = AutoTraitor
spell-magic-missile = Magic Missile
spell-disable-tech = Disable Tech
spell-blink = Blink
spell-teleport = Teleport
spell-forcewall = Forcewall
spell-fireball = Fireball
spell-knock = Knock
spell-invisible-wall = Invisible Wall
spell-blind = Blind
spell-mutate = Mutate
spell-blind-eyes = Your eyes cry out in pain!
spell-mutate-strong = You feel strong!
spell-recharging = { $name } is still recharging.
spell-incapacitated = Not when you're incapacitated.
spell-cant-speak = You can't get the words out!
spell-no-robe = I don't feel strong enough without my robe.
spell-no-sandals = I don't feel strong enough without my sandals.
spell-no-hat = I don't feel strong enough without my hat.
spell-learned = You know { $name } now.
spell-title = Spells
spell-line = { $name } ({ $value }/{ $max })
spell-choose-area = Where to?
spell-choose-target = Who?
spell-cancel = Cancel
# The wizard's things (spells.md)
scroll-learned = You have learned { $spell }!
orb-see-everything = You can see... everything!
orb-stares = { $name } stares into the scrying orb, their eyes glazing over.
survivor-you = You are a survivor! Your own safety comes first: trust no one, and kill anyone who gets in your way. Armed as you are, now would be the time to settle an old score...
book-title = The Book of Spells
book-uses = Spells left to memorize: { $value }
book-choose = Memorize which spell? The number after each is its recharge, in seconds.
book-line = { $name } ({ $value })
book-line-once = { $name } (once only, on everyone)
book-magic-missile = { spell-magic-missile }
book-fireball = { spell-fireball }
book-disable-tech = { spell-disable-tech }
book-forcewall = { spell-forcewall }
book-blink = { spell-blink }
book-teleport = { spell-teleport }
book-knock = { spell-knock }
book-blind = { spell-blind }
book-mutate = { spell-mutate }
book-summon-guns = Summon Guns
book-armor = Mastercrafted Armour Set
book-scrying = Scrying Orb
book-desc-magic-missile = Slow magic bolts at everyone nearby. Each knocks down and lightly burns whoever it reaches.
book-desc-fireball = A fireball the way you face. No robes needed. Mind anyone standing beside you.
book-desc-disable-tech = A pulse that knocks out weapons, cameras and most other machines around you.
book-desc-forcewall = A wall nothing breaks, for 30 seconds. No robes needed.
book-desc-blink = A short jump somewhere at random: for getting away, or, with patience, getting in.
book-desc-teleport = To somewhere in a place of your choosing. Good in a tight spot, but slow to recharge.
book-desc-knock = Opens the doors around you. No robes needed.
book-desc-blind = Blinds one person for a while. No robes needed.
book-desc-mutate = For a short while you are a hulk, see through walls, shrug off cold and fire, and need no air.
book-desc-summon-guns = Arms a crew that was only waiting for an excuse. Mind the crossfire.
book-desc-armor = Armour you can cast in, that guards you better and holds out space.
book-desc-scrying = An orb of crackling light: stare into it to leave your body and roam the ship as a ghost.
book-artefacts = Artefacts
book-artefacts-note = Powerful things steeped in magic. Each counts against the spells you can learn. Best left to experienced wizards.
book-rememorize = Re-memorize spells
book-known = You already know that spell.
book-learned = You have learned { $spell }.
book-summoned-guns = You have cast Summon Guns.
book-bought-armor = You have bought a suit of wizard armour.
book-bought-orb = You have bought a scrying orb.
book-rememorized = Every spell is gone from your mind. You may now learn a new set.
book-rememorize-elsewhere = You can only re-memorize spells in the wizard's sanctuary.
admin-no-spell = There's no spell called { $name }. Spells: { $text }.
vote-choice-derelict = Derelict
job-scavenger = Scavenger

## Genetics (`genetics.md`)
gene-no-breath = You stop breathing.
gene-remote-view = Your mind expands.
gene-regenerate = You feel strange.
gene-run = You feel quick.
gene-remote-talk = You expand your mind outwards.
gene-morph = Your skin feels strange.
gene-blend = You feel alone.
gene-hallucination = Your mind says 'Hello'.
gene-no-prints = Your fingers feel numb.
gene-shock-immune = You feel strange.
gene-small = Your skin feels rubbery.
gene-hulk = Your muscles hurt.
gene-headache = You get a headache.
gene-fake = You feel strange.
gene-cough = You start coughing.
gene-clumsy = You feel lightheaded.
gene-twitch = You twitch.
gene-xray = The walls suddenly disappear.
gene-nervous = You feel nervous.
gene-cold-resist = Your body feels warm.
gene-blind = You can't seem to see anything.
gene-telekinesis = You feel smarter.
gene-deaf = It's kinda quiet...
gene-hulk-lost = You suddenly feel very weak.
gene-seizure = You have a seizure!
gene-seizure-seen = { $name } starts having a seizure!
radiation-weak = You feel weak.
radiation-mutate = You mutate!
injector-trying = { $name } is trying to inject { $victim } with the DNA-Injector!
injector-injected = { $victim } has been injected with the DNA-Injector by { $name }.
injector-you-inject = You inject { $victim }.
genetics-no-occupant = No viable occupant detected.
genetics-transferred = Transferred.
genetics-replicator-not-ready = Replicator not ready yet.
genetics-injector-error = Error in injector creation.
genetics-injector-made = Injector created.
genetics-inaprovaline = Occupant now has { $value } units of inaprovaline in their bloodstream.
genetics-occupant = Occupant Statistics:
genetics-health = Health %: { $value } ({ $stat })
genetics-stat-conscious = Conscious
genetics-stat-unconscious = Unconscious
genetics-stat-dead = *dead*
genetics-radiation = Radiation Level: { $value }%
genetics-ue = Unique Enzymes: { $value }
genetics-ui = Unique Identifier: { $value }
genetics-se = Structural Enzymes: { $value }
genetics-ruined = The occupant's DNA structure is ruined beyond recognition, please insert a subject with an intact DNA structure.
genetics-empty = The scanner is empty.
genetics-no-scanner = Error: No DNA Modifier connected.
genetics-modify-ui = Modify Unique Identifier
genetics-modify-se = Modify Structural Enzymes
genetics-buffers-menu = View/Edit/Transfer Buffer
genetics-pulse = Pulse Radiation
genetics-emitter-menu = Radiation Emitter Settings
genetics-inject = Inject Inaprovaline
genetics-lock = Lock (Unlocked)
genetics-unlock = Unlock (Locked)
genetics-main-menu = Main Menu
genetics-buffers-link = Buffers Menu
genetics-working = Working ... Please wait ({ $value } Seconds)
genetics-duration = Radiation Duration: { $value }
genetics-strength = Radiation Intensity: { $value }
genetics-duration-label = Duration
genetics-strength-label = Intensity
genetics-block = Selected Block: { $value }
genetics-sub = Selected Sub-Block: { $value }
genetics-target = Selected Target: { $value }
genetics-block-label = Block
genetics-sub-label = Sub-Block
genetics-target-label = Target
genetics-modify-block = Modify Block:
genetics-irradiate = Irradiate
genetics-buffer = Buffer { $value }:
genetics-buffer-empty = Buffer Empty
genetics-buffer-data = Data: { $value }
genetics-buffer-by = By: { $value }
genetics-label-ui = Label: Unique Identifier
genetics-label-uiue = Label: Unique Identifier & Unique Enzymes
genetics-label-se = Label: Structural Enzymes
genetics-save = Save:
genetics-save-ui = UI
genetics-save-uiue = UI+UE
genetics-save-se = SE
genetics-transfer-to = Transfer to:
genetics-to-occupant = Occupant
genetics-full-injector = Full Injector
genetics-iso-injector = Iso Injector
genetics-iso-pick = Block for the injector:
genetics-clear = Clear Buffer

## Teleporters (`teleporters.md`)
tele-locked-in = Locked In
tele-engaged = Teleporter engaged!
tele-disengaged = Teleporter disengaged!
tele-no-lock = Failure: Cannot authenticate locked on coordinates. Please reinstate coordinate matrix.
tele-recharging = The hand teleporter is recharging!
tele-row-active = { $value } (Active)
tele-row-inactive = { $value } (Inactive)
tele-row-none = None (Dangerous)
## Random events (`events.md`)
command-alert = NanoTrasen Update. { $title }: { $text }
alert-anomaly = Anomaly Alert
alert-meteor = Meteor Alert
alert-maintenance = Maintenance alert
alert-medical = Medical Alert
event-storm-start = The ship is flying through an electrical storm. Radio communications may be disrupted.
event-storm-end = The ship has cleared the electrical storm. Radio communications restored.
event-meteors-start = The ship is now travelling through a meteor shower.
event-meteors-end = The ship has cleared the meteor shower.
event-portals-start = Subspace disruption detected around the vessel.
event-grid-check = The ship is performing an automated power system grid check, please stand by.
event-radiation-start = The ship is now travelling through a radiation belt.
event-radiation-end = The ship has cleared the radiation belt.
admin-events-on = Events toggled on by { $name }.
admin-events-off = Events toggled off by { $name }.
admin-events-disabled = Events are not enabled.
admin-event-started = Started event: { $name }.
admin-event-busy = There is an active event.
admin-no-event = There's no event called { $name }. Events: { $text }.
## Sinks, showers, toilets and mirrors (`plumbing.md`)
sink-busy = Someone's already washing here.
sink-start-hands = You start washing your hands.
sink-start-item = You start washing the { NOUN($item) }.
sink-washed-hands = { $name } washes their hands using the { NOUN($sink) }.
sink-washed-item = { $name } washes { NOUN($item, case: "indef") } using the { NOUN($sink) }.
sink-washed-item-self = You wash { NOUN($item, case: "indef") } using the { NOUN($sink) }.
sink-fills = { $name } fills the { NOUN($item) } using the { NOUN($sink) }.
sink-fills-self = You fill the { NOUN($item) } using the { NOUN($sink) }.
shower-reads = The water temperature seems to be { $temp }.
shower-temp-normal = normal
shower-temp-freezing = freezing
shower-temp-boiling = boiling
shower-valve-start = You begin to adjust the temperature valve with the wrench.
shower-adjusted = { $name } adjusts the shower with the wrench.
shower-adjusted-self = You adjust the shower with the wrench.
shower-freezing = The water is freezing!
shower-searing = The water is searing!
toilet-tighter-grip = You need a tighter grip.
toilet-slam = { $name } slams { $victim } into the toilet!
toilet-slam-self = You slam { $victim } into the toilet!
mirror-smashed = { $name } smashes the mirror with the { NOUN($item) }!
mirror-hit = { $name } hits the mirror with the { NOUN($item) }!

# Surgery (surgery.md)
surgery-appendectomy = Appendectomy
surgery-brain-removal = Brain removal
surgery-eye-surgery = Eye surgery
surgery-plastic-surgery = Plastic surgery
surgery-gender-reassignment = Gender reassignment
surgery-cavity-implant = Cavity implant
surgery-expose = You need to expose their { NOUN($zone) } first.
surgery-draped = { $name } drapes { $target }'s { NOUN($zone) } to prepare for { $procedure }.
surgery-laid-on-table = { $target } has been laid on the operating table by { $name }.
surgery-begin-incise = { $name } begins to make an incision in { $target }'s { NOUN($zone) }.
surgery-begin-clamp = { $name } begins to clamp bleeders in { $target }'s { NOUN($zone) }.
surgery-begin-retract = { $name } begins to retract the skin in { $target }'s { NOUN($zone) }.
surgery-begin-close = { $name } begins to mend the incision in { $target }'s { NOUN($zone) }.
surgery-begin-saw = { $name } begins to saw through the bone in { $target }'s { NOUN($zone) }.
surgery-begin-extract-appendix = { $name } begins to extract { $target }'s appendix.
surgery-begin-look-appendix = { $name } looks for an appendix in { $target }.
surgery-begin-extract-brain = { $name } begins to extract { $target }'s brain.
surgery-begin-look-brain = { $name } looks for a brain in { $target }'s head.
surgery-begin-fix-eyes = { $name } begins to fix { $target }'s eyes.
surgery-begin-reshape-face = { $name } begins to alter { $target }'s appearance.
surgery-begin-reshape-genitals = { $name } begins to reshape { $target }'s genitals to look more feminine.
surgery-begin-reshape-genitals-male = { $name } begins to reshape { $target }'s genitals to look more masculine.
surgery-begin-cavity = { $name } begins to insert something into { $target }'s { NOUN($zone) }.
surgery-begin-cavity-check = { $name } checks for items in { $target }'s { NOUN($zone) }.
surgery-done-incise = { $name } succeeds!
surgery-done-clamp = { $name } succeeds!
surgery-done-retract = { $name } succeeds!
surgery-done-close = { $name } succeeds!
surgery-done-saw = { $name } succeeds!
surgery-done-extract-appendix = { $name } successfully removes { $target }'s appendix!
surgery-no-appendix = { $name } can't find an appendix in { $target }!
surgery-done-extract-brain = { $name } successfully removes { $target }'s brain!
surgery-no-brain = { $name } can't find a brain in { $target }!
surgery-done-fix-eyes = { $name } successfully fixes { $target }'s eyes!
surgery-failed-fix-eyes = { $name } accidentally stabs { $target } right in the head!
surgery-done-reshape-face = { $name } alters { $target }'s appearance completely, they are now { $new }!
surgery-done-reshape-genitals-male = { $name } has made a man of { $target }!
surgery-done-reshape-genitals-female = { $name } has made a woman of { $target }!
surgery-failed-reshape-genitals = { $name } mutilates { $target }'s genitals beyond the point of recognition!
surgery-cavity-no-fit = { $name } can't seem to fit it in { $target }'s { NOUN($zone) }.
surgery-cavity-stuffed = { $name } stuffs something into { $target }'s { NOUN($zone) }!
surgery-cavity-pulled = { $name } pulls something out of { $target }'s { NOUN($zone) }!
surgery-cavity-empty = { $name } doesn't find anything in { $target }'s { NOUN($zone) }.
surgery-failed = { $name } screws up!
