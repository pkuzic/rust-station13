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
shields-raised = Вы поднимаете щиты станции.
shields-lowered = Вы опускаете щиты станции.
shields-no-charge = Ничего не происходит: в конденсаторах щита слишком мало заряда.
shield-converter-mode = Вы { $mode ->
    [manual] переводите преобразователь в ручной режим
    [auto] переводите преобразователь в автоматический режим
   *[off] выключаете преобразователь
}.
shield-capacitor-charge = Заряжен на { $value }%.
hud-health-ghost = Вы призрак. Нажмите R, чтобы начать заново новым персонажем
ghost-left-body = Вы покидаете своё тело. Живые вас больше не видят и не слышат.
respawn-no-room = Все точки появления заняты. Попробуйте чуть позже.
ghost-say = [МЁРТВЫЕ] { $name } { $verb ->
    [ask] спрашивает
    [exclaim] восклицает
   *[say] говорит
}: «{ $text }»
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
        [glasses] Очки
        [ears] Уши
       *[id] ID
    }

# Подсказки HUD. Аргументы: $what.
hud-tip = { $what ->
        [hand-left] Левая рука
        [hand-right] Правая рука
        [swap] Сменить руку
        [drop] Бросить
        [pull] Отпустить
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

# Кабели и мотки кабеля. Аргументы: $name (кто), $item (id существительного), $count (отрезков кабеля).
cable-too-far = Так далеко кабель не проложить.
cable-floor-intact = Здесь не проложить кабель, пока не снята плитка пола.
cable-exists = Здесь уже есть кабель.
cable-cut = { $name } перерезает { NOUN($item, case: "acc") }.
coil-cut-piece = Вы отрезаете кусок от мотка кабеля.
coil-joined = Вы сматываете кабель в один моток.
coil-transferred = Вы перематываете { $count } { $count ->
        [one] отрезок
        [few] отрезка
       *[many] отрезков
    } кабеля с одного мотка на другой.
coil-full = Моток слишком большой, больше кабеля в него не поместится.
examine-coil-short = Короткий кусок силового кабеля.
examine-coil-piece = Кусок силового кабеля.
examine-coil = Моток силового кабеля. В нём { $count } { $count ->
        [one] отрезок
        [few] отрезка
       *[many] отрезков
    } кабеля.
table-disassembling = Вы разбираете стол.
table-disassembling-reinforced = Вы разбираете укреплённый стол.
table-weakening = Вы ослабляете укреплённый стол.
table-weakened = Стол ослаблен.
table-strengthening = Вы укрепляете укреплённый стол.
table-strengthened = Стол укреплён.
meter-reading = Давление: { $value } кПа
meter-no-pipe = Результат: ошибка подключения!
meter-too-far = Вы слишком далеко.
examine-meter = Измеритель потока газа. Манометр показывает { $value } кПа.
examine-meter-no-pipe = Измеритель потока газа. Мигает лампа ошибки подключения.
welder-on = Теперь вы будете варить при ударе.
welder-off = Вы больше не варите.
welder-no-fuel = Нужно больше топлива!
welder-short = Нужно больше сварочного топлива!
welder-refueled = Сварочный аппарат заправлен.
examine-welder = В нём осталось { $count } { $count ->
        [one] единица
        [few] единицы
       *[many] единиц
    } топлива!
examine-fuel-tank = В нём { $count } { $count ->
        [one] единица
        [few] единицы
       *[many] единиц
    } сварочного топлива.
examine-floor-tiles = В стопке { $count } { $count ->
        [one] плитка
        [few] плитки
       *[many] плиток
    } пола.

# Освещение (docs/reference/lights.md).
light-no-fitting = В светильнике нет { NOUN($item, case: "gen") }.
light-removed = Вы вынимаете { NOUN($item, case: "acc") }.
light-burn-hand = Вы пытаетесь вынуть { NOUN($item, case: "acc") }, но обжигаете руку!
light-smash-burn = Вы разбиваете { NOUN($item, case: "acc") }, но обжигаете руку!
light-inserted = Вы вставляете { NOUN($item, case: "acc") }.
light-replaced = Вы заменяете { NOUN($item, case: "acc") }.
light-wrong-type = Сюда подходит только { NOUN($item) }.
light-stick = Вы суёте { NOUN($item, case: "acc") } в патрон светильника!
light-hit = Вы бьёте по светильнику!
light-smashed = Вы бьёте по светильнику, и лампа разбивается!
light-smashed-seen = { $name } разбивает светильник!
examine-light-on = Он включён.
examine-light-off = Он выключен.
examine-light-empty = Лампы в нём нет.
examine-light-burned = Лампа в нём перегорела.
examine-light-broken = Лампа в нём разбита.

# Шкафы, шкафчики и ящики (docs/reference/closets.md).
closet-wont-budge = Не поддаётся!
closet-welded = { $name } заваривает { NOUN($item, case: "acc") }.
closet-unwelded = { $name } срезает сварной шов с { NOUN($item, case: "gen") }.
closet-cut-apart = { $name } разрезает { NOUN($item, case: "acc") } сварочным аппаратом.
locker-locked = { $name } запирает шкафчик.
locker-unlocked = { $name } отпирает шкафчик.
locker-access-denied = Доступ запрещён
locker-locked-inside = Шкафчик заперт!

# Медицина (medical.md). Аргументы: $name (кто лечит), $target, $item (id существительного), числа.
medical-applied = { $name } применяет { NOUN($item, case: "acc") } на { $target }
medical-applied-self = { $name } применяет { NOUN($item, case: "acc") } на себе
analyzer-used = { $name } проверяет жизненные показатели { $target }.
analyzer-used-self = Вы проверили жизненные показатели { $target }.
analyzer-status = Результаты анализа { $target }: общее состояние: { $dead ->
        [yes] мёртв
       *[no] здоров на { $health }%
    }
analyzer-damage = Ключ: удушье/токсины/ожоги/травмы. Повреждения: { $oxy } - { $tox } - { $burn } - { $brute }
analyzer-temperature = Температура тела: { $celsius }°C ({ $fahrenheit }°F)
analyzer-localized = Повреждения по частям тела, травмы/ожоги:
analyzer-organ = { NOUN($zone) }: { $brute } - { $burn }
analyzer-limbs-ok = Конечности в порядке.
analyzer-summary = { $oxy ->
        [severe] Обнаружено тяжёлое кислородное голодание
       *[ok] Уровень кислорода в крови в норме
    } | { $tox ->
        [severe] Обнаружено опасное количество токсинов
       *[ok] Уровень токсинов в крови минимален
    } | { $burn ->
        [severe] Обнаружены тяжёлые ожоги
       *[ok] Ожогов нет
    } | { $brute ->
        [severe] Обнаружены тяжёлые травмы
       *[ok] Травм нет
    }

# Пожарная сигнализация (fire-alarm.md). Аргументы: $name, $item (id существительного).
fire-alarm-raised = Вы включаете пожарную тревогу.
fire-alarm-reset = Вы сбрасываете пожарную тревогу.
fire-alarm-disconnected = { $name } отключает датчик { NOUN($item, case: "gen") }!
fire-alarm-reconnected = { $name } снова подключает датчик { NOUN($item, case: "gen") }!

# Переносной генератор (generators.md). Аргументы: $count, $watts, $on (yes/no).
pacman-fuel-added = Вы загружаете в генератор { $count } { $count ->
    [one] лист
    [few] листа
   *[many] листов
} плазмы.
pacman-full = Генератор полон!
pacman-secured = Вы прикручиваете генератор к полу.
pacman-unsecured = Вы откручиваете генератор от пола.
pacman-on = Вы включаете генератор.
pacman-off = Вы выключаете генератор.
pacman-no-fuel = В генераторе нет топлива.
examine-pacman = Генератор { $on ->
    [yes] включён
   *[no] выключен
}. В нём { $count } { $count ->
    [one] лист
    [few] листа
   *[many] листов
} плазмы, мощность { $watts } Вт.

# Шкаф для баллонов (tank-dispenser.md). Аргументы: $item (id существительного), $count.
dispenser-taken = Вы достаёте { NOUN($item, case: "acc") } из шкафа для баллонов.
dispenser-put = Вы кладёте { NOUN($item, case: "acc") } в шкаф для баллонов.
dispenser-full = Шкаф для баллонов полон.
dispenser-empty = Шкаф для баллонов пуст.
examine-tank-dispenser = В нём { $count } { $count ->
    [one] кислородный баллон
    [few] кислородных баллона
   *[many] кислородных баллонов
}.

# Батареи и зарядные устройства (cells.md). Аргументы: $name, $rating, $value, $charge (-1: нет).
charger-inserted-self = Вы вставляете батарею в зарядное устройство.
charger-inserted = { $name } вставляет батарею в зарядное устройство.
charger-removed-self = Вы вынимаете батарею из зарядного устройства.
charger-removed = { $name } вынимает батарею из зарядного устройства.
charger-occupied = В зарядном устройстве уже есть батарея.
charger-blinks = Зарядное устройство мигает красным, когда вы пытаетесь вставить батарею!
charger-remove-cell-first = Сначала выньте батарею!
charger-attached = Вы прикручиваете зарядное устройство к полу.
charger-detached = Вы откручиваете зарядное устройство от пола.
examine-cell = Судя по этикетке, ёмкость батареи — { $rating }. Индикатор заряда показывает { $value }%.
examine-cell-charger = { $charge ->
    [-1] В зарядном устройстве нет батареи.
   *[other] В зарядном устройстве батарея. Текущий заряд: { $charge }.
}

# Реактивные ранцы (jetpacks.md).
jetpack-on = Вы включаете реактивный ранец.
jetpack-off = Вы выключаете реактивный ранец.

# Рация (radio.md). Аргументы: $name, $frequency («145.9»), $verb (say/ask/exclaim), $text.
radio-say = [{ $frequency }] { $name } { $verb ->
        [ask] спрашивает
        [exclaim] восклицает
       *[say] говорит
    }: «{ $text }»
radio-microphone-on = Вы включаете микрофон.
radio-microphone-off = Вы выключаете микрофон.

# Стулья (chairs.md). Аргументы: $name, $target (кто сидел).
chair-buckled-self = Вы пристёгиваетесь к { NOUN($seat, case: "dat") }.
chair-buckles = { $name } пристёгивается!
chair-unbuckled-self = Вы отстёгиваетесь от { NOUN($seat, case: "gen") }.
chair-unbuckles = { $name } отстёгивается.
chair-unbuckled-by = { $name } отстёгивает { $target }!

# Энергощиток (power.md): щелчки.
apc-close-cover-first = Сначала закройте крышку, чтобы провести картой.
apc-interface-locked = Вы блокируете интерфейс энергощитка.
apc-interface-unlocked = Вы разблокируете интерфейс энергощитка.
apc-cover-opened = Вы открываете крышку энергощитка.
apc-cover-closed = Вы закрываете крышку энергощитка.
apc-cover-locked = Крышка заперта и не открывается.
apc-has-cell = Батарея уже установлена.
apc-cell-inserted = Вы вставляете батарею.
apc-cell-removed = Вы вынимаете батарею.
# Окно энергощитка. Аргументы: $area, $on (yes/no), $state, $value, $auto (yes/no), $load (Вт),
# $mode (0 выкл., 1 авто-выкл., 2 вкл., 3 авто-вкл.).
apc-title = Энергощиток ({ $area })
apc-hint-locked = (Проведите картой, чтобы разблокировать интерфейс.)
apc-hint-unlocked = (Проведите картой, чтобы заблокировать интерфейс.)
apc-breaker = Главный выключатель: { $on ->
    [yes] вкл.
   *[no] выкл.
}
apc-external = Внешнее питание: { $state ->
    [2] в норме
    [1] слабое
   *[0] нет
}
apc-cell = Батарея: { $value }% ({ $state ->
    [1] заряжается
    [2] полностью заряжена
   *[0] не заряжается
}) ({ $auto ->
    [yes] авто
   *[no] выкл.
})
apc-no-cell = Батарея: не подключена.
apc-equipment = Оборудование: { $load } Вт: { $mode ->
    [1] выкл. (авто)
    [2] вкл.
    [3] вкл. (авто)
   *[0] выкл.
}
apc-lighting = Освещение: { $load } Вт: { $mode ->
    [1] выкл. (авто)
    [2] вкл.
    [3] вкл. (авто)
   *[0] выкл.
}
apc-environment = Жизнеобеспечение: { $load } Вт: { $mode ->
    [1] выкл. (авто)
    [2] вкл.
    [3] вкл. (авто)
   *[0] выкл.
}
apc-cover-lock = Замок крышки: { $on ->
    [yes] закрыт
   *[no] открыт
}
apc-toggle = Переключить
apc-set-off = Выкл.
apc-set-on = Вкл.
apc-set-auto = Авто

# Компьютер контроля энергосети (power.md). Аргументы: $supply, $load (Вт); $area;
# $equipment, $lighting, $environment (0 выкл., 1 авто-выкл., 2 вкл., 3 авто-вкл.);
# $value (%); $state (0 не заряжается, 1 заряжается, 2 заряжена).
monitor-title = Контроль энергосети
monitor-totals = Всего мощности: { $supply } Вт. Общая нагрузка: { $load } Вт.
monitor-apc = { $area }: { $equipment ->
    [1] АВыкл
    [2] Вкл
    [3] АВкл
   *[0] Выкл
} / { $lighting ->
    [1] АВыкл
    [2] Вкл
    [3] АВкл
   *[0] Выкл
} / { $environment ->
    [1] АВыкл
    [2] Вкл
    [3] АВкл
   *[0] Выкл
}, { $load } Вт, батарея { $value }% { $state ->
    [1] З
    [2] П
   *[0] Н
}
monitor-apc-no-cell = { $area }: { $equipment ->
    [1] АВыкл
    [2] Вкл
    [3] АВкл
   *[0] Выкл
} / { $lighting ->
    [1] АВыкл
    [2] Вкл
    [3] АВкл
   *[0] Выкл
} / { $environment ->
    [1] АВыкл
    [2] Вкл
    [3] АВкл
   *[0] Выкл
}, { $load } Вт, нет батареи

# Термоэлектрический генератор (engine.md). Аргументы: $watts; $hot, $cold (K); $hotkpa, $coldkpa.
examine-teg = Мощность: { $watts } Вт. Горячий контур: { $hot } K, { $hotkpa } кПа. Холодный контур: { $cold } K, { $coldkpa } кПа.

# Ядро двигателя (engine.md). Окно управления лазерами; $on (yes/no). Предупреждения ядра
# по рации; $frequency («145.9»).
laser-title = Состояние лазеров
laser-none = Лазеры не найдены
laser-power = Мощность:
laser-output = Излучение: { $on ->
    [yes] включено
   *[no] выключено
}
core-overload = [{ $frequency }] Компьютер управления ядром: ПЕРЕГРУЗКА ЯДРА
core-explosion-imminent = [{ $frequency }] Компьютер управления ядром: ВЗРЫВ ЯДРА НЕМИНУЕМ
core-safe = [{ $frequency }] Компьютер управления ядром: ядро возвращается к безопасному режиму.

# Окно охладителя (engine.md). Аргументы: $on (yes/no), $temperature (K), $pressure (кПа).
freezer-status = Состояние: { $on ->
    [yes] включён
   *[no] выключен
}
freezer-gas = Газ: { $temperature } K, { $pressure } кПа
freezer-setting = Целевая температура газа:

# Окно накопителя (power.md). Аргументы: $value (%), $charging, $auto, $on (yes/no), $load (Вт).
smes-title = Накопитель энергии
smes-charge = Заряд: { $value }%
smes-input = Вход: { $charging ->
    [yes] заряжается
   *[no] не заряжается
} ({ $auto ->
    [yes] авто
   *[no] выкл.
})
smes-input-level = Уровень входа:
smes-output = Выход: { $on ->
    [yes] включён
   *[no] выключен
}
smes-output-level = Уровень выхода:
smes-load = Нагрузка выхода: { $load } Вт

# Настенный воспламенитель (igniters.md). Аргументы: $name, $item (id существительного).
sparker-disabled = { $name } отключает { NOUN($item, case: "acc") }!
sparker-disabled-self = Вы отсоединяете { NOUN($item, case: "acc") }.
sparker-reconnected = { $name } снова подключает { NOUN($item, case: "acc") }!
sparker-reconnected-self = Вы восстанавливаете подключение { NOUN($item, case: "gen") }.

# Аварийный прожектор (floodlights.md).
floodlight-on = Вы включаете прожектор.
floodlight-off = Вы выключаете прожектор.
floodlight-cell-removed = Вы вынимаете батарею.
floodlight-unscrewed = Вы откручиваете крышку батарейного отсека.
floodlight-screwed = Вы прикручиваете крышку батарейного отсека.
floodlight-panel-removed = Вы снимаете крышку батарейного отсека.
floodlight-panel-replaced = Вы ломом ставите крышку батарейного отсека на место.
floodlight-cell-already = Батарея уже установлена.
floodlight-cell-inserted = Вы вставляете батарею.

# Мусоропровод (disposals.md). Аргументы: $name, $item, $bin (id существительных).
disposal-placed-self = Вы кладёте { NOUN($item, case: "acc") } в { NOUN($bin, case: "acc") }.
disposal-placed = { $name } кладёт { NOUN($item, case: "acc") } в { NOUN($bin, case: "acc") }.

# Огнетушители (extinguishers.md). Аргументы: $count; $item, $cabinet (id существительных).
extinguisher-safety-on = Предохранитель включён.
extinguisher-safety-off = Предохранитель снят.
extinguisher-empty = Огнетушитель пуст.
extinguisher-refilled = Огнетушитель заправлен.
extinguisher-taken = Вы достаёте { NOUN($item, case: "acc") } из { NOUN($cabinet, case: "gen") }.
extinguisher-placed = Вы кладёте { NOUN($item, case: "acc") } в { NOUN($cabinet, case: "acc") }.
examine-extinguisher = В нём осталось { $count } { $count ->
        [one] единица
        [few] единицы
       *[many] единиц
    } воды!
examine-water-tank = В нём { $count } { $count ->
        [one] единица
        [few] единицы
       *[many] единиц
    } воды.

# Меню правой кнопки. Аргументы: $item (id существительного).
menu-thing = { NOUN($item) }
menu-examine = Осмотреть
menu-use = Использовать
menu-pull = Тянуть

# Аварийные настенные шкафчики (walllockers.md). Аргументы: $first, $second, $third, $item (id существительных).
walllocker-empty = Пусто...
walllocker-dispensed-1 = Выдано: { NOUN($first) }.
walllocker-dispensed-2 = Выдано: { NOUN($first) } и { NOUN($second) }.
walllocker-dispensed-3 = Выдано: { NOUN($first) }, { NOUN($second) } и { NOUN($third) }.
walllocker-put-back = Вы кладёте { NOUN($item, case: "acc") } обратно в шкафчик.

# Окна канистры, переносного насоса и скруббера. Аргументы: $value, $connected, $on, $out (yes/no).
gas-title-canister = Канистра
gas-title-pump = Переносной воздушный насос
gas-title-scrubber = Переносной воздушный скруббер
gas-pressure = Давление: { $value } кПа
gas-port = Порт: { $connected ->
        [yes] подключён
       *[no] отключён
    }
gas-tank = Давление в баллоне: { $value } кПа
gas-remove-tank = Вынуть баллон
gas-valve = Выпускной клапан: { $on ->
        [yes] открыт
       *[no] закрыт
    }
gas-power = Питание: { $on ->
        [yes] вкл.
       *[no] выкл.
    }
gas-direction = Направление: { $out ->
        [yes] наружу
       *[no] внутрь
    }
gas-release = Давление выпуска:
gas-target = Целевое давление:
gas-rate = Производительность:

# Окно торгового автомата (vending.md).
vending-select = Выберите товар:
vending-vend = Купить
vending-sold-out = Распродано

# Еда и питьё (food.md). Аргументы: $item, $tool (id существительных); $verb; $name, $target;
# $count; $level.
food-eat-gobble = Вы жадно { food-verb } { NOUN($item, case: "acc") } и проглатываете!
food-eat-begin = Вы жадно принимаетесь { food-verb-inf } { NOUN($item, case: "acc") }.
food-eat = Вы { food-verb } { NOUN($item, case: "acc") }.
food-eat-unwilling = Вы нехотя { food-verb } немного { NOUN($item, case: "gen") }.
food-eat-full = Вы больше не можете запихнуть в себя { NOUN($item, case: "acc") }.
food-verb = { $verb ->
        [chew] жуёте
        [nibble] покусываете
        [gnaw] грызёте
        [gobble] уплетаете
        [chomp] чавкаете
       *[bite] откусываете
    }
food-verb-inf = { $verb ->
        [chew] жевать
        [nibble] покусывать
        [gnaw] грызть
        [gobble] уплетать
        [chomp] чавкать
       *[bite] откусывать
    }
drink-gulp = Вы делаете глоток { NOUN($item, case: "gen") }.
drink-empty = { NOUN($item) } — пусто, вот незадача!
drink-feeds = { $name } поит { $target }: { NOUN($item) }.
examine-bitten-once = { NOUN($item) } — кто-то уже надкусил!
examine-bitten = { NOUN($item) } — надкушено { $count } раза!
examine-bitten-many = { NOUN($item) } — надкушено много раз!
examine-drink = { NOUN($item) }: { $level ->
        [empty] пусто!
        [almost-empty] почти пусто!
        [half] наполовину полно!
        [almost-full] почти полно!
       *[full] полно!
    }
hunger-1 = Вы чувствуете голод
hunger-2 = Вас мучает жажда
hunger-3 = Пожалуй, стоит перекусить
hunger-4 = У вас урчит в животе
hunger-5 = Пожалуй, стоит чего-нибудь выпить...
hunger-6 = Вы чувствуете пустоту внутри
hunger-7 = Вы немного проголодались
hunger-8 = То, что вы ели в последний раз, не очень-то насытило...
hunger-9 = Хм, пицца была бы кстати

# Курение (smoking.md).
cigarettes-out = Сигареты кончились, чёрт! Как теперь дожить до конца дня...
examine-cigarettes = Осталось сигарет: { $count }!
lighter-on = После нескольких попыток { $name } удаётся зажечь { NOUN($item, case: "acc") }.
lighter-on-burnt = После нескольких попыток { $name } удаётся зажечь { NOUN($item, case: "acc") }, но при этом обжигает палец.
lighter-burn-self = Вы обжигаетесь, зажигая зажигалку.
lighter-off = { $name } тихо гасит { NOUN($item, case: "acc") }.
cigarette-lit-welder = { $name } небрежно прикуривает { NOUN($item, case: "acc") } от { NOUN($tool, case: "gen") }. Крутой.
cigarette-lit-lighter = Повозившись, { $name } прикуривает { NOUN($item, case: "acc") } от { NOUN($tool, case: "gen") }.
cigarette-lit = { $name } прикуривает { NOUN($item, case: "acc") } от { NOUN($tool, case: "gen") }.
cigarette-out = Ваша { NOUN($item) } гаснет.
cigarette-trodden = { $name } спокойно бросает { NOUN($item, case: "acc") } и затаптывает её.

# Действия с предметами в меню правой кнопки (verbs.rs).
verb-toggle-open = Открыть/закрыть
verb-toggle-lock = Запереть/отпереть
verb-rotate-chair = Повернуть стул
verb-toggle-light = Включить/выключить свет
verb-toggle-jetpack = Включить/выключить ранец

# Камеры наблюдения (cameras.md). Аргументы: $name; $item (id существительного).
camera-deactivated = { $name } отключает { NOUN($item, case: "acc") }!
camera-reactivated = { $name } снова включает { NOUN($item, case: "acc") }!
camera-static = Экран покрывается помехами.
cameras-title = Камеры наблюдения
cameras-select = На какую камеру переключиться?
cameras-view = Смотреть
cameras-cancel = Отмена
cameras-deactivated = { " " }(отключена)

# Бумаги (paperwork.md). Аргументы: $item (id существительного); $count.
paper-bin-take = Вы достаёте { NOUN($item, case: "acc") } из лотка.
paper-bin-put = Вы кладёте { NOUN($item, case: "acc") } в лоток.
paper-bin-empty = Лоток пуст!
examine-paper-bin = { $count ->
        [0] В лотке нет бумаги.
        [one] В лотке { $count } лист.
        [few] В лотке { $count } листа.
       *[many] В лотке { $count } листов.
    }
clipboard-clipped = Вы прикрепляете лист к { NOUN($item, case: "dat") }.
clipboard-pen = Вы вставляете { NOUN($item, case: "acc") } в планшет.
clipboard-top = Вы перекладываете { NOUN($item, case: "acc") } наверх.
clipboard-title = Планшет
clipboard-remove-pen = Вынуть ручку
clipboard-add-pen = Вставить ручку
clipboard-write = Писать
clipboard-remove = Вынуть
clipboard-top-button = Наверх
paper-write-prompt = Что вы хотите написать?
paper-write-keys = (Enter — новая строка, Ctrl+Enter — сохранить, Esc — отмена.)
paper-label-prompt = Как подписать лист?
paper-save = Сохранить

# Картотеки (filing-cabinets.md). Аргументы: $cabinet, $item (id существительных).
cabinet-put = Вы кладёте { NOUN($item, case: "acc") } в { NOUN($cabinet, case: "acc") }.
cabinet-wont-fit = Нельзя положить { NOUN($item, case: "acc") } в { NOUN($cabinet, case: "acc") }!
cabinet-wrenched = Вы прикручиваете { NOUN($cabinet, case: "acc") } к полу.
cabinet-unwrenched = Вы откручиваете { NOUN($cabinet, case: "acc") } от пола.
cabinet-empty = В { NOUN($cabinet, case: "prep") } пусто.
pick-list-title = { NOUN($item) }

# Простыни (bedsheets.md). Аргументы: $item (id существительного); $count.
linen-bin-put = Вы кладёте { NOUN($item, case: "acc") } в корзину для белья.
linen-bin-hide = Вы прячете { NOUN($item, case: "acc") } среди простыней.
linen-bin-take = Вы достаёте { NOUN($item, case: "acc") } из корзины для белья.
linen-bin-falls-out = Из простыни выпадает { NOUN($item) }!
examine-linen-bin = { $count ->
        [0] В корзине нет простыней.
        [one] В корзине { $count } простыня.
        [few] В корзине { $count } простыни.
       *[many] В корзине { $count } простыней.
    }

# Обогреватели (space-heaters.md). Аргументы: $name; $on, $open (yes, no); $cell (yes, no,
# meter); $value.
heater-hatch-closed = Чтобы вставить батарею, откройте крышку.
heater-has-cell = Внутри уже есть батарея.
heater-cell-in = Вы вставляете батарею в обогреватель.
heater-cell-in-seen = { $name } вставляет батарею в обогреватель.
heater-cell-out = Вы вынимаете батарею из обогревателя.
heater-cell-out-seen = { $name } вынимает батарею из обогревателя.
heater-hatch-opened = Вы открываете крышку обогревателя.
heater-hatch-opened-seen = { $name } открывает крышку обогревателя.
heater-hatch-shut = Вы закрываете крышку обогревателя.
heater-hatch-shut-seen = { $name } закрывает крышку обогревателя.
heater-switched-on = Вы включаете обогреватель.
heater-switched-on-seen = { $name } включает обогреватель.
heater-switched-off = Вы выключаете обогреватель.
heater-switched-off-seen = { $name } выключает обогреватель.
examine-heater = Обогреватель { $on ->
        [yes] включён
       *[no] выключен
    }, крышка { $open ->
        [yes] открыта
       *[no] закрыта
    }. { $cell ->
        [yes] Батарея на месте.
        [no] Батареи нет.
       *[meter] Индикатор заряда показывает { $value }%.
    }
heater-title = Панель управления обогревателем
heater-cell = Батарея:
heater-cell-installed = Установлена
heater-cell-removed = Снята
heater-level = Заряд: { $value }%
heater-set = Температура:
heater-degrees = { $value }°C

# Морг (morgue.md). Аргументы: $name, $victim (имена); $item (id существительного).
morgue-stuffed = { $name } заталкивает { $victim } на { NOUN($item, case: "acc") }!
crematorium-locked = Заперто.
crematorium-denied = Доступ запрещён.
crematorium-crackle = Слышен глухой треск.
crematorium-roar = Слышен рёв: крематорий запускается.
