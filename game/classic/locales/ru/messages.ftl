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

# Подсказки HUD. Аргументы: $what.
hud-tip = { $what ->
        [hand-left] Левая рука
        [hand-right] Правая рука
        [swap] Сменить руку
        [drop] Бросить
        [intent] Намерение
        [move] Бег / шаг
        [internals] Баллон
        [oxygen] Кислород
        [toxin] Токсины
        [fire] Жар
        [temperature] Температура тела
       *[health] Здоровье
    }

# Панель техобслуживания шлюза и провода. Аргументы: $item (id предмета), $name, $colour.
door-panel-opened = Вы открываете панель техобслуживания { NOUN($item, case: "gen") }.
door-panel-closed = Вы закрываете панель техобслуживания { NOUN($item, case: "gen") }.
wire-need-cutters = Нужны кусачки!
wire-need-multitool = Нужен мультитул!
wire-pulse-cut = На перерезанный провод импульс не подать.
shocked = Вас пронзает мощный разряд тока!
shocked-seen = { $name } бьёт током!
panel-title = Панель доступа
panel-wire = { $colour ->
        [orange] Оранжевый
        [dark-red] Тёмно-красный
        [white] Белый
        [yellow] Жёлтый
        [red] Красный
        [blue] Синий
        [green] Зелёный
        [grey] Серый
        [black] Чёрный
        [pink] Розовый
       *[brown] Коричневый
    } провод:
panel-cut = Перерезать
panel-mend = Починить
panel-pulse = Импульс
panel-close = Закрыть
panel-bolts-down = Болты двери опущены!
panel-bolts-up = Болты двери подняты.
panel-test-light-on = Контрольная лампа горит.
panel-test-light-off = Контрольная лампа не горит!
panel-ai-light-on = Лампа «Управление ИИ разрешено» горит.
panel-ai-light-off = Лампа «Управление ИИ разрешено» не горит.
panel-safety-flashing = Лампа безопасности мигает!
panel-safety-on = Лампа безопасности горит.
panel-hydraulics-green = Лампа гидравлики горит ровным зелёным.
panel-hydraulics-red = Лампа гидравлики мигает красным.

# Воздушная сигнализация. Аргументы: $item (id существительного), $name, $area, $value.
alarm-deactivated = { $name } отключает { NOUN($item, case: "acc") }!
alarm-reactivated = { $name } снова включает { NOUN($item, case: "acc") }!
alarm-title = Атмосфера: { $area }
alarm-pressure = Давление: { $value } кПа
alarm-nitrogen = Азот: { $value }%
alarm-oxygen = Кислород: { $value }%
alarm-co2 = CO₂: { $value }%
alarm-plasma = Плазма: { $value }%
alarm-unknown = Неизвестный газ: { $value }%
alarm-temperature = Температура: { $value } °C
alarm-seal-on = Включить аварийную герметизацию
alarm-seal-off = Отключить аварийную герметизацию
alarm-panic-on = Включить аварийную подачу воздуха
alarm-panic-off = Отключить аварийную подачу воздуха
alarm-no-vents = Вентиляция не подключена!

# Названия зон.
area-engineering = Инженерный отсек
area-eng-storage = Инженерный склад
# Инженерная секция NSV Luna.
area-engine-hallway = Коридор двигателя
area-engine-control = Управление двигателем
area-engine-monitoring = Наблюдение за двигателем
area-engine-combustion = Камера сгорания
area-engine-walls = Стены двигателя
area-electrical-storage = Электрический склад
area-chief-engineer = Кабинет главного инженера
area-maint-fore-starboard = Носовое правое техобслуживание инженерной палубы
area-maint-starboard = Правое техобслуживание нижней палубы
area-maint-port = Левое техобслуживание инженерной палубы
area-maint-port-central = Центральное левое техобслуживание инженерной палубы
area-shield-generation = Генератор щитов
area-escape-hangar = Ангар эвакуации экипажа

# Контейнеры. Аргументы: $name, $item и $target (id существительных).
storage-added = { $name } кладёт { NOUN($item, case: "acc") } в { NOUN($target, case: "acc") }!
storage-cannot-hold = В { NOUN($target, case: "acc") } нельзя положить { NOUN($item, case: "acc") }.
hud-storage-close = Закрыть
