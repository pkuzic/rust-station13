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
