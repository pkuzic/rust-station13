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
apc-breaker-on = You switch the APC's main breaker on.
apc-breaker-off = You switch the APC's main breaker off.
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
       *[id] ID
    }

# HUD hover tips (Luna's screen object names). Args: $what.
hud-tip = { $what ->
        [hand-left] Left hand
        [hand-right] Right hand
        [swap] Swap hands
        [drop] Drop
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
