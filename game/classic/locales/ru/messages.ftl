# Args: $user, $target (English names, never declined), $user_gender (male/female),
# $weapon (noun id).
attack-with = { $user } { $user_gender ->
        [female] ударила
       *[other] ударил
    } { $target } { NOUN($weapon, case: "ins") }.

# Args: $item (noun id).
item-fell = { NOUN($item) } { GENDER($item) ->
        [feminine] упала
        [neuter] упало
        [plural] упали
       *[masculine] упал
    } на пол.

# Args: $item (noun id), $count (number).
stack-count = { $count } { $count ->
        [one] { NOUN($item) }
        [few] { NOUN($item, case: "gen") }
       *[many] { NOUN($item, case: "gen", number: "pl") }
    }

# Args: $user, $user_gender, $item (noun id), $target (noun id).
use-on = { $user } { $user_gender ->
        [female] открыла
       *[other] открыл
    } { NOUN($target, case: "acc") } с помощью { NOUN($item, case: "gen") }.

# Args: $item (noun id).
examine = Это { NOUN($item) }.

airlock-pushed = Шлюз выталкивает вас с дороги!

# Hands bar labels.
hud-hand-left = Л
hud-hand-right = П

# Args: $name (English, never declined).
examine-person = Это { $name }.

# Args: $name, $verb (say/ask/exclaim), $text (the player's own words, any language).
say = { $name } { $verb ->
        [ask] спрашивает
        [exclaim] восклицает
       *[say] говорит
    }: «{ $text }»

# Args: $name (the name the player keeps instead).
name-rejected = Это имя недопустимо. Вас зовут { $name }.

# Prompt of the say line.
hud-say = Сказать:

# Бой. Аргументы: $attacker, $victim (имена), $item (id предмета), $zone (id части тела).
# Настоящее время, чтобы не согласовывать глагол с полом.
attacked-with = { $attacker } бьёт { $victim } { NOUN($item, case: "ins") } в { NOUN($zone, case: "acc") }!
punched = { $attacker } бьёт { $victim } кулаком!
punched-face = { $attacker } бьёт { $victim } кулаком в лицо!
punch-missed = { $attacker } пытается ударить { $victim }, но промахивается!

# Строка состояния. Аргументы: $value (здоровье), $intent (help/harm).
hud-health = Здоровье: { $value }
hud-health-crit = Здоровье: КРИТИЧЕСКОЕ
hud-health-dead = Вы мертвы
hud-intent = Намерение: { $intent ->
        [harm] вред
       *[help] помощь
    }

# Окружение. Аргументы: $value (кПа).
gasp = Вы задыхаетесь!
hud-pressure = Давление: { $value } кПа
alert-vacuum = ВАКУУМ
alert-low-pressure = Низкое давление
alert-high-pressure = Высокое давление
alert-low-oxygen = Не хватает кислорода
alert-cold = Вы замерзаете
alert-hot = Вы перегреваетесь

# Оборудование. Аргументы: $item (id предмета), $value (кПа).
switched-on = Вы включаете { NOUN($item, case: "acc") }.
switched-off = Вы выключаете { NOUN($item, case: "acc") }.
valve-opened = Вы открываете { NOUN($item, case: "acc") }.
valve-closed = Вы закрываете { NOUN($item, case: "acc") }.
canister-connected = Вы подключаете { NOUN($item, case: "acc") } к порту.
canister-disconnected = Вы отключаете { NOUN($item, case: "acc") } от порта.
examine-gauge = Манометр показывает { $value } кПа.

# Питание. Аргументы: $value (проценты); $equipment, $lighting, $environment (on/off); $item (id).
examine-apc = Заряд батареи энергощитка: { $value }%. Оборудование: { $equipment ->
        [on] вкл.
       *[off] выкл.
    }, освещение: { $lighting ->
        [on] вкл.
       *[off] выкл.
    }, жизнеобеспечение: { $environment ->
        [on] вкл.
       *[off] выкл.
    }.
examine-smes = Накопитель энергии заряжен на { $value }%.
apc-breaker-on = Вы включаете главный рубильник энергощитка.
apc-breaker-off = Вы выключаете главный рубильник энергощитка.
apc-locked = Доступ запрещён.
door-forced = Вы вскрываете { NOUN($item, case: "acc") } ломом.

# Снаряжение. Аргументы: $item (id предмета).
equip-wrong-slot = Это сюда не надеть.
equip-occupied = Здесь уже что-то надето.
equip-needs-uniform = Для этого нужен комбинезон.
internals-on = Вы дышите из { NOUN($item, case: "gen") }.
internals-off = Вы больше не дышите из баллона.
internals-no-mask = На вас нет дыхательной маски.
internals-no-tank = Вам не к чему подключиться: нет баллона.

# Панель снаряжения.
hud-internals = Баллон
hud-slot = { $slot ->
        [uniform] Форма
        [suit] Костюм
        [head] Голова
        [mask] Маска
        [gloves] Перчатки
        [shoes] Обувь
        [back] Спина
        [belt] Пояс
       *[id] ID
    }
