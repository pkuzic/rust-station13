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
