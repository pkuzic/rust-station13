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
flash-blinds = { $name } ослепляет { $victim } вспышкой!
flash-click = *щёлк* *щёлк*
flash-burnt-out = Лампа перегорела!
flasher-disconnected = Вы отсоединяете лампу вспышки.
flasher-connected = Вы подсоединяете лампу вспышки.
flasher-secured = Теперь она закреплена.
flasher-loose = Теперь её можно двигать.
monkey-emote = { $name } { $act ->
    [jump] прыгает!
    [roll] катается по полу.
    [tail] машет хвостом.
   *[scratch] чешется.
}
microwave-on = Микроволновка включается.
microwave-added = Вы кладёте это в микроволновку.
microwave-full = Микроволновка полна.
microwave-unsuitable = Вы не представляете, что из этого можно приготовить.
arcade-villain = { $title ->
    [automatic] Автоматический
    [farmer] Фермер
    [lord] Лорд
    [professor] Профессор
    [evil] Злой
    [dread-king] Ужасный Король
   *[space] Космический
} { $villain ->
    [melonoid] Мелоноид
    [murdertron] Убийтрон
    [sorcerer] Колдун
    [ruin] Руин
    [jeff] Джефф
    [ectoplasm] Эктоплазм
    [crushulon] Крушулон
   *[coder] Кодер
}
arcade-new = Победители не употребляют космонаркотики
arcade-new-round = Новый раунд
arcade-attack = Вы наносите { $value } урона!
arcade-heal = Вы тратите { $cost } магии и лечите { $value } урона!
arcade-recharge = Вы восстанавливаете { $value } очков
arcade-won = { $enemy } повержен! Ликуйте!
arcade-steal = { $enemy } крадёт { $value } вашей силы!
arcade-drained = Вас осушили! ИГРА ОКОНЧЕНА
arcade-enemy-heals = { $enemy } лечит { $value } здоровья!
arcade-enemy-attacks = { $enemy } наносит { $value } урона!
arcade-crushed = Вас раздавили! ИГРА ОКОНЧЕНА
arcade-stats = Здоровье: { $hp } | Магия: { $mp } | Здоровье врага: { $enemyhp }
arcade-button-attack = Атаковать
arcade-button-heal = Лечиться
arcade-button-recharge = Восстановить силы
arcade-button-new = Новая игра
airlock-cycling = Шлюз начинает цикл.
scanner-occupied = Сканер уже занят!
cloning-scanned = Образец успешно отсканирован.
cloning-already-scanned = Образец уже есть в базе.
cloning-no-genes = Ошибка: не удаётся найти генетические данные.
cloning-no-pod = Не удаётся начать цикл клонирования: рядом нет капсулы.
cloning-busy = Не удаётся начать цикл клонирования: капсула занята.
cloning-nobody = Сейчас некого клонировать из базы.
cloning-started = Цикл клонирования начат.
cloning-begins = Начат процесс создания клона. Подождите немного.
cloning-done = Капсула открывается. Вы снова живы.
brig-title = Таймер двери
brig-time = Осталось: { $minutes }:{ $seconds }
brig-start = Запустить таймер и закрыть дверь
brig-stop = Остановить таймер и открыть дверь
brig-flash = Включить вспышку
brig-flash-charging = Вспышка заряжается
brig-denied = Доступ запрещён.
gun-fires = { $name } стреляет из { NOUN($item, case: "gen") }!
gun-click = *щёлк*
gun-set-kill = Теперь он настроен на убийство.
gun-set-stun = Теперь он настроен на оглушение.
shot-hit-laser = В { $name } попадает лазер!
shot-hit-electrode = В { $name } попадает электрод, и цель падает!
baton-on = Дубинка включена.
baton-off = Дубинка выключена.
baton-empty = Дубинка разряжена.
baton-prod = { $name } тычет дубинкой в { $victim }. К счастью, она выключена.
baton-stun = { $name } оглушает { $victim } шоковой дубинкой!
recharger-no-power = Зарядник мигает красным, когда вы вставляете это.
cuffs-applying = { $name } пытается надеть наручники на { $victim }!
cuffs-applied = { $name } надевает наручники на { $victim }!
cuffs-removing = { $name } пытается снять наручники с { $victim }.
cuffs-removed = { $name } снимает наручники с { $victim }.
cuffs-resisting = { $name } пытается освободиться от наручников!
cuffs-broken-out = { $name } освобождается от наручников!
turret-denied = Доступ запрещён.
turret-set-stun = Турели включены и настроены на оглушение.
turret-set-lethal = Турели включены и настроены на убийство.
turret-set-off = Турели выключены.
syringe-no-blood = Вам не удаётся найти кровь.
syringe-nothing = Набирать нечего, или шприц полон.
syringe-drawn = Вы набираете в шприц { $value } ед.
syringe-empty = Он пуст.
syringe-injecting = { $name } пытается сделать укол { $victim }!
syringe-injected = Вы вводите { $value } ед.
hypo-injected = Вы вводите гипоспреем { $value } ед.
pill-swallowed = Вы глотаете таблетку.
pill-fed = Вы заставляете { $victim } проглотить таблетку.
glass-poured = Вы переливаете { $value } ед.
glass-nothing = Переливать нечего, или нет места.
reagents-contains = Внутри { $value } ед.
reagents-empty = Внутри пусто.
reagent-aluminium = Алюминий
reagent-carbon = Углерод
reagent-chlorine = Хлор
reagent-copper = Медь
reagent-ethanol = Этанол
reagent-fluorine = Фтор
reagent-hydrogen = Водород
reagent-iron = Железо
reagent-lithium = Литий
reagent-mercury = Ртуть
reagent-nitrogen = Азот
reagent-oxygen = Кислород
reagent-phosphorus = Фосфор
reagent-potassium = Калий
reagent-radium = Радий
reagent-sacid = Серная кислота
reagent-silicon = Кремний
reagent-sodium = Натрий
reagent-sugar = Сахар
reagent-sulfur = Сера
reagent-water = Вода
dispenser-title = Химический раздатчик. Энергия: { $value }
dispenser-beaker = Стакан: { $value } из { $max } ед.
dispenser-no-beaker = Стакан не вставлен.
dispenser-eject = Вынуть стакан
dispenser-occupied = В машине уже есть стакан.
reagent-anti_toxin = Антитоксин
reagent-kelotane = Келотан
reagent-tricordrazine = Трикордразин
reagent-dexalinp = Дексалин плюс
reagent-synaptizine = Синаптизин
reagent-cryoxadone = Криоксадон
reagent-toxin = Токсин
reagent-nutriment = Питательные вещества
reagent-gold = Золото
reagent-silver = Серебро
reagent-plasma = Плазма
reagent-inaprovaline = Инапровалин
reagent-stoxin = Снотворное
reagent-dermaline = Дермалин
reagent-bicaridine = Бикаридин
reagent-dexalin = Дексалин
medbay-put-in = { $name } помещает { $victim } внутрь.
medbay-occupied = Там уже кто-то есть.
medbay-beaker-full = Стакан уже вставлен.
sleeper-numb = Вас окутывает прохладный воздух. Вы цепенеете, чувства притупляются.
sleeper-empty = Капсула пуста.
sleeper-occupant = { $name }: { $state ->
    [conscious] в сознании
    [unconscious] без сознания
   *[dead] МЁРТВ
}
sleeper-readings = Здоровье: { $value }. Кислородный урон: { $oxy }. Токсины: { $tox }.
sleeper-inject = Ввести: { $reagent } (сейчас { $value } ед.)
sleeper-unstable = ОШИБКА: состояние пациента не позволяет автоматическую инъекцию.
sleeper-cant-metabolise = ОШИБКА: организм пациента не усваивает препараты.
cryo-on = Вы включаете криокамеру.
cryo-off = Вы выключаете криокамеру.
master-title = ХимМастер 3000
master-line = { $reagent }: { $value } ед.
master-eject = Извлечь стакан и очистить буфер
master-to-buffer = Стакан (нажмите, чтобы перенести 10 ед. в буфер):
master-to-beaker = Буфер (нажмите, чтобы вернуть 10 ед.):
master-pill = Сделать таблетку (до 50 ед.)
master-bottle = Сделать бутылочку (до 30 ед.)
tank-filled = Вы наполняете сосуд: { $value } ед.
tank-empty = Там пусто.
tank-glass-full = Сосуд уже полон.
vending-denied = Доступ запрещён.
reagent-condensedcapsaicin = Концентрированный капсаицин
reagent-beer = Пиво
reagent-fuel = Сварочное топливо
gun-loaded = Вы заряжаете оружие, патронов: { $value }.
gun-full = Оружие уже заряжено полностью.
gun-wrong-ammo = Эти патроны не подходят.
gun-other-ammo = Заряжены патроны другого типа. Сначала отстреляйте их.
ammo-empty = Там ничего не осталось.
shot-hit-bullet = В { $name } попадает пуля!
lathe-title = Металл: { $metal } см³. Стекло: { $glass } см³.
lathe-busy = Работает...
lathe-row = { NOUN($item) } ({ $metal } м, { $glass } с)
lathe-fed = Загружено листов: { $value }.
lathe-full = В нём нет места для этого.
lathe-refuses = Это туда не загрузить.
scanner-empty = Сканер пуст.
scanner-unhurt = Ран не обнаружено.
scanner-part = { NOUN($item) }: ушибы { $value }, ожоги { $burn }.
job-captain = Капитан
job-hop = Глава персонала
job-hos = Глава службы безопасности
job-chief-engineer = Главный инженер
job-research-director = Директор исследований
job-security-officer = Сотрудник службы безопасности
job-warden = Надзиратель
job-forensic-technician = Криминалист
job-engineer = Инженер
job-atmos-technician = Атмосферный техник
job-medical-doctor = Врач
job-chemist = Химик
job-geneticist = Генетик
job-scientist = Учёный
job-roboticist = Робототехник
job-hydroponicist = Гидропонист
job-barman = Бармен
job-chef = Повар
job-janitor = Уборщик
job-quartermaster = Завхоз
job-cargo-technician = Грузчик
job-shaft-miner = Шахтёр
job-counselor = Советник
job-clown = Клоун
job-mime = Мим
job-assistant = Ассистент
job-assigned = Ваша должность: { $job ->
    [captain] Капитан
    [hop] Глава персонала
    [hos] Глава службы безопасности
    [chief-engineer] Главный инженер
    [research-director] Директор исследований
    [security-officer] Сотрудник службы безопасности
    [warden] Надзиратель
    [forensic-technician] Криминалист
    [engineer] Инженер
    [atmos-technician] Атмосферный техник
    [medical-doctor] Врач
    [chemist] Химик
    [geneticist] Генетик
    [scientist] Учёный
    [roboticist] Робототехник
    [hydroponicist] Гидропонист
    [barman] Бармен
    [chef] Повар
    [janitor] Уборщик
    [quartermaster] Завхоз
    [cargo-technician] Грузчик
    [shaft-miner] Шахтёр
    [counselor] Советник
    [clown] Клоун
    [mime] Мим
   *[assistant] Ассистент
}.
job-window-title = Вам предложена должность: { $job }. Согласитесь или выберите другую свободную.
job-keep = Начать на этой должности
job-row = { $job } (свободно: { $value })
records-security = Записи службы безопасности. Нажмите на человека, чтобы изменить статус.
records-medical = Медицинские записи
records-empty = Записей нет.
records-row = { $name }, { $job }: { $status }
records-denied = Доступ запрещён.
record-none = Нет
record-arrest = *Арестовать*
record-incarcerated = В заключении
record-parolled = Условно освобождён
record-released = Освобождён
record-active = Активен
record-unconscious = *Без сознания*
record-deceased = *Мёртв*
id-computer-empty = Вставьте карту: нажмите на компьютер, держа её в руке.
id-computer-card = Карта: { $name }. Должность: { $job }.
id-computer-no-job = нет
id-computer-eject = Извлечь карту
examine-card = Владелец: { $name }, { $job ->
    [captain] Капитан
    [hop] Глава персонала
    [hos] Глава службы безопасности
    [chief-engineer] Главный инженер
    [research-director] Директор исследований
    [security-officer] Сотрудник службы безопасности
    [warden] Надзиратель
    [forensic-technician] Криминалист
    [engineer] Инженер
    [atmos-technician] Атмосферный техник
    [medical-doctor] Врач
    [chemist] Химик
    [geneticist] Генетик
    [scientist] Учёный
    [roboticist] Робототехник
    [hydroponicist] Гидропонист
    [barman] Бармен
    [chef] Повар
    [janitor] Уборщик
    [quartermaster] Завхоз
    [cargo-technician] Грузчик
    [shaft-miner] Шахтёр
    [counselor] Советник
    [clown] Клоун
    [mime] Мим
   *[assistant] Ассистент
}.
pot-occupied = В нём уже что-то растёт.
pot-planted = Вы сажаете семена.
pot-cleared = Вы убираете погибшее растение.
pot-harvested = Вы собираете урожай: { $value } шт.
pot-not-ready = Урожай ещё не созрел.
examine-pot-empty = В нём ничего не растёт. Воды: { $value } ед.
examine-pot-dead = Растение в нём погибло.
examine-pot-ripe = Растение готово к сбору урожая. Воды: { $value } ед.
examine-pot-growing = В нём что-то растёт. Воды: { $value } ед.
barrier-locked = Замок барьера включён.
barrier-unlocked = Замок барьера выключен.
barrier-repaired = Вы чините барьер.
barrier-hit = Вы бьёте по барьеру.
barrier-broken = Барьер разваливается.
apc-hand-full = Чтобы работать с ЛКП, нужна свободная рука.
round-mode = Текущий режим игры: { $mode ->
    [traitor] Предатель
    [meteor] Метеоры
    [malfunction] Сбой ИИ
    [nuclear] Ядерная угроза
    [revolution] Революция
    [blob] Блоб
    [changeling] Генокрад
    [rp-revolution] Ролевая революция
    [monkey] Обезьяны
    [autotraitor] Автопредатель
    [derelict] Заброшенная станция
    [restructuring] Корпоративная реструктуризация
    [sandbox] Песочница
   *[extended] Расширенный
}. Приятной игры!
lobby-title = Добро пожаловать в лобби!
lobby-countdown = Раунд начнётся через { $value } с. Готовы: { $ready } из { $players }.
lobby-ready = Я готов
lobby-unready = Я не готов
lobby-round-on = Раунд уже идёт.
lobby-join = Войти в игру
lobby-name = Ваше имя: { $name }. Нажмите T, чтобы изменить.
lobby-no-name = Имя будет случайным. Нажмите T, чтобы ввести своё.
lobby-prefer = Желаемая должность:
comms-title = Пульт связи
comms-call = Вызвать спасательные капсулы
comms-recall = Остановить запуск
comms-launching = Спасательные капсулы стартуют через { $value } с.
comms-gone = Спасательные капсулы улетели.
comms-denied = Доступ запрещён.
pods-called = Внимание: спасательные капсулы стартуют через { $value } мин. Их двери открыты.
pods-already-called = Запуск уже начат!
pods-recalled = Внимание: запуск спасательных капсул отменён.
pods-minutes = До старта спасательных капсул: { $value } мин.
pods-seconds = До старта спасательных капсул: { $value } с.
pods-launched = Спасательные капсулы стартовали!
round-over = Раунд окончен. Перезапуск через 60 секунд.
round-escaped = { $name }, { $job }: спасся.
round-stranded = { $name }, { $job }: остался на станции.
round-died = { $name }, { $job }: погиб.
id-manifest-open = Открыть список экипажа
id-manifest-title = Список экипажа:
id-manifest-hint = Менять записи можно на компьютере охраны.
id-manifest-row = { $name } - { $job }
id-manifest-print = Распечатать
id-manifest-printing = Идёт печать...
id-manifest-back = Назад
id-manifest-paper = бумага - «Список экипажа»
traitor-you = Вы предатель.
traitor-repeat = ПОВТОРЯЮ
traitor-reminder = На случай, если вы прослушали: ВЫ ПРЕДАТЕЛЬ!
traitor-uplink-pack = Синдикат спрятал корабельную рацию в вашем рюкзаке. Используйте её в руке: это аплинк, и там же можно вспомнить задания.
traitor-uplink-hand = Синдикат вложил корабельную рацию вам в руку. Используйте её в руке: это аплинк, и там же можно вспомнить задания.
traitor-uplink-floor = Синдикат оставил корабельную рацию у ваших ног. Используйте её в руке: это аплинк, и там же можно вспомнить задания.
objective-steal = Задание №{ $value }: украсть { NOUN($item, "acc") }.{ $done ->
    [won] {" "}Успех
    [failed] {" "}Провал
   *[none] {""}
}
objective-assassinate = Задание №{ $value }: убить: { $name }, { $job }.{ $done ->
    [won] {" "}Успех
    [failed] {" "}Провал
   *[none] {""}
}
objective-protect = Задание №{ $value }: { $name }, { $job } — родственник высокопоставленного лидера Синдиката. Позаботьтесь, чтобы он благополучно покинул станцию.{ $done ->
    [won] {" "}Успех
    [failed] {" "}Провал
   *[none] {""}
}
objective-escape = Задание №{ $value }: улететь на капсуле живым и не под арестом.{ $done ->
    [won] {" "}Успех
    [failed] {" "}Провал
   *[none] {""}
}
objective-hijack = Задание №{ $value }: угнать капсулы: улететь должны только вы.{ $done ->
    [won] {" "}Успех
    [failed] {" "}Провал
   *[none] {""}
}
uplink-title = Консоль аплинка Синдиката:
uplink-crystals = Осталось телекристаллов: { $value }
uplink-good = { NOUN($item) } ({ $value })
round-traitor = Предателем Синдиката был { $name }.
round-traitor-won = Предатель справился!
round-traitor-lost = Предатель провалился!
vote-started = Началось голосование { $kind ->
    [mode] за смену режима
   *[restart] за перезапуск
}. Осталось секунд: { $value }. Нажмите V, чтобы проголосовать.
vote-title = Идёт голосование { $kind ->
    [mode] за смену режима
   *[restart] за перезапуск
}.
vote-closes-in = До конца голосования секунд: { $value }.
vote-next-in = Следующее голосование можно начать через секунд: { $value }.
vote-leading = Сейчас побеждает: { $choice }
vote-begin-restart = Начать голосование за перезапуск
vote-begin-mode = Начать голосование за смену режима
vote-choice-no = Нет
vote-choice-restart = Да
vote-choice-default = Без изменений
vote-choice-secret = Секретный
vote-choice-extended = Расширенный
vote-choice-traitor = Предатель
vote-result-no-restart = Голосование окончено. Перезапуска не будет.
vote-result-restart = Голосование окончено. Сервер перезапустится через секунд: { $value }.
vote-result-no-change = Голосование окончено. Без изменений.
vote-result-mode = Голосование окончено. Режим меняется на: { $mode }.
vote-reboot = Сервер перезапустится через секунд: { $value }.
lobby-vote = Голосование
emote-blink = { $name } моргает.
emote-blink-r = { $name } часто моргает.
emote-wave = { $name } машет рукой.
emote-choke = { $name } задыхается!
emote-clap = { $name } хлопает в ладоши.
emote-drool = { $name } пускает слюни.
emote-eyebrow = { $name } поднимает бровь.
emote-chuckle = { $name } посмеивается.
emote-twitch = { $name } сильно дёргается.
emote-twitch-s = { $name } дёргается.
emote-faint = { $name } падает в обморок.
emote-cough = { $name } кашляет!
emote-frown = { $name } хмурится.
emote-nod = { $name } кивает.
emote-blush = { $name } краснеет.
emote-gasp = { $name } хватает ртом воздух!
emote-giggle = { $name } хихикает.
emote-grin = { $name } ухмыляется.
emote-cry = { $name } плачет.
emote-sigh = { $name } вздыхает.
emote-laugh = { $name } смеётся.
emote-mumble = { $name } бормочет!
emote-grumble = { $name } ворчит!
emote-groan = { $name } стонет!
emote-howl = { $name } воет!
emote-moan = { $name } охает!
emote-raise = { $name } поднимает руку.
emote-shrug = { $name } пожимает плечами.
emote-smile = { $name } улыбается.
emote-shiver = { $name } дрожит.
emote-pale = { $name } на миг бледнеет.
emote-tremble = { $name } трясётся от страха!
emote-sneeze = { $name } чихает.
emote-sniff = { $name } шмыгает носом.
emote-snore = { $name } храпит.
emote-whimper = { $name } хнычет.
emote-wink = { $name } подмигивает.
emote-yawn = { $name } зевает.
emote-collapse = { $name } падает без сил!
emote-scream = { $name } кричит!
emote-point = { $name } показывает пальцем.
emote-me = { $name } { $text }
emote-unknown = Такого действия нет.
whisper = { $name } шепчет: «{ $text }»
whisper-seen = { $name } что-то шепчет.
ooc-say = OOC: { $name }: { $text }
ooc-unnamed = Кто-то
chat-prompt-ooc = OOC:
chat-prompt-me = Я:
channel-science = Наука
channel-command = Командование
channel-medical = Медицина
channel-engineering = Инженерия
channel-security = Охрана
channel-syndicate = Синдикат
channel-supply = Снабжение
protected-head = Удар в голову не прошёл: вас защитило снаряжение.
protected-chest = Удар в грудь не прошёл: вас защитило снаряжение.
protected-groin = Удар в пах не прошёл: вас защитило снаряжение.
knocked-out = { $victim }: без сознания!
knocked-down = { $victim }: сбит с ног!
stunned = { $victim }: оглушён!
punch-weakened = { $attacker } ослабляет противника: { $victim }!
punch-knocked-down = { $attacker } сбивает с ног: { $victim }!
punch-stunned = { $attacker } оглушает: { $victim }!
pushed-down = { $attacker } толкает на пол: { $victim }!
disarmed = { $attacker } выбивает предмет из рук: { $victim }!
disarm-missed = { $attacker } пытается выбить предмет из рук: { $victim }!
shakes-awake = { $attacker } трясёт, пытаясь разбудить: { $victim }!
checks-self = { $name } осматривает себя.
self-part-ok = { NOUN($zone) }: в порядке.
self-part-one = { NOUN($zone) }: { $a }.
self-part-two = { NOUN($zone) }: { $a } и { $b }.
status-bruised = ушиб
status-mangled = тяжёлая рана
status-numb = онемение
status-blistered = волдыри
status-peeling = кожа слезает
grab-passive = { $attacker } хватает: { $victim }!
grab-aggressive = { $attacker } крепко хватает обеими руками: { $victim }!
grab-failed = { $attacker } не удаётся крепко схватить: { $victim }!
grab-neck = { $attacker } берёт за шею: { $victim }!
grab-choke = { $attacker } сжимает шею: { $victim }!
grab-loosen = { $attacker } ослабляет хватку на шее: { $victim }!
grab-broke-free = { $victim } вырывается из хватки: { $attacker }!
grab-broke-headlock = { $victim } вырывается из захвата шеи: { $attacker }!
grab-on-table = { $attacker } кладёт на стол: { $victim }.
grab-take-off = Сначала надо снять: { NOUN($item, "acc") }!
armor-absorbs = Броня поглощает удар!
armor-softens = Броня смягчает удар!
slipped = Вы поскальзываетесь: { NOUN($item) }!
explosion-heard = Станцию сотрясает взрыв!
build-title = Что сделать из листов ({ $value }):
build-recipe = { NOUN($item) } (листов: { $cost })
build-recipe-many = { NOUN($item) }, { $made } шт. (листов: { $cost })
build-need-more = Материала не хватает.
build-no-room = Здесь нет места.
build-too-many-panes = Здесь слишком много окон.
build-need-fuel = Для этого нужно больше топлива.
build-unwelding = Вы снимаете обшивку стены.
build-unwelded = Вы сняли обшивку стены.
build-interrupted = Вы прекращаете работу.
admin-your-key = Ваш ключ: { $key }.
admin-not-admin = Вы не администратор.
admin-low-rank = Для этого нужен ранг выше.
admin-unknown = Нет такой команды: { $verb }. Попробуйте help.
admin-help = Команды: who, asay ТЕКСТ, dsay ТЕКСТ, pm КТО ТЕКСТ, announce ТЕКСТ, startnow, toggleooc, togglespawning, jump X Y [Z], jumpto КТО, kick КТО [ЗА ЧТО], ban КТО МИНУТЫ [ЗА ЧТО] (0 навсегда), unban КЛЮЧ, bans, restart, mute КТО, explode РАЗМЕР, emp СИЛЬНО СЛАБО, spell КТО ЗАКЛИНАНИЕ, toggleevents, event [СОБЫТИЕ], get КТО, spawn ЧТО, delete ЧТО, god КТО, rejuv КТО.
admin-welcome = Вы администратор. Нажмите F2 для строки команд и введите help.
admin-no-one = Нет никого по имени { $name }.
admin-no-thing = Здесь нет такого: { $name }.
admin-asay = АДМИН: { $name }: { $text }
ghost-dsay = [МЁРТВЫЕ] Админ { $name }: { $text }
admin-pm-from = Сообщение от админа { $name }: { $text }
admin-pm-to = Сообщение от { $name } для { $target }: { $text }
admin-announce = Важное объявление: { $text }
admin-started = { $name } начинает раунд.
admin-ooc-on = OOC включён.
admin-ooc-off = OOC выключен.
admin-kicked = Вас выгнал администратор { $name }. { $text }
admin-banned = Вас забанил администратор { $name }. { $text }
admin-unbanned = С ключа { $key } снят бан.
admin-no-ban = На ключе { $key } нет бана.
admin-ban-row = { $key }, выдал { $by }, { $value ->
    [-1] навсегда
   *[other] осталось минут: { $value }
}: { $text }
admin-muted = Вам запретили говорить.
admin-voiced = Вам снова разрешили говорить.
admin-did = { $name }: { $verb }, цель: { $target }.
admin-who-row = { $name } ({ $key }), { $job }, { $address }
admin-help-from = ПОМОЩЬ: { $name } ({ $key }): { $text }
admin-help-sent = Сообщение админам: { $text }
chat-prompt-ahelp = Вопрос админам:
chat-prompt-admin = Админ:
comms-red-on = Объявить красную тревогу
comms-red-off = Отменить красную тревогу
comms-display-title = Что показывать на экранах:
comms-display-pods = Отсчёт до старта капсул
comms-display-blank = Очистить
comms-display-message = Сообщение
comms-picture-default = Тревога: нет
comms-picture-redalert = Тревога: красная
comms-picture-lockdown = Тревога: блокировка
comms-picture-biohazard = Тревога: биоугроза
chat-prompt-display = Строка 1/Строка 2:
rwall-step-0 = Вы перерезаете опорные тросы.
rwall-step-1 = Вы снимаете опорные тросы.
rwall-step-2 = Вы разрезаете металлический кожух.
rwall-step-3 = Вы отгибаете кожух.
rwall-step-4 = Вы отсоединяете опорные прутья.
rwall-step-5 = Вы снимаете опорные прутья.
rwall-step-6 = Вы отдираете внешнюю обшивку.
hair-hair_a = Короткие
hair-hair_b = Длинные
hair-hair_longfringe = Длинная чёлка
hair-hair_vlongfringe = Очень длинная чёлка
hair-hair_longest = Самая длинная чёлка
hair-hair_test = Женственная
hair-hair_c = Стрижка
hair-hair_d = Ирокез
hair-hair_e = Лысеющие
hair-hair_f = Волна
hair-hair_bedhead = Растрёпанные
hair-hair_bedheadv2 = Растрёпанные 2
hair-hair_spikey = Торчком
hair-hair_dreads = Дреды
hair-hair_afro = Афро
hair-hair_bigafro = Большое афро
hair-hair_braid = Коса
hair-hair_kagami = Кагами
hair-hair_ponytail = Хвост
hair-facial_watson = Ватсон
hair-facial_chaplin = Чаплин
hair-facial_selleck = Селлек
hair-facial_neckbeard = Шейная борода
hair-facial_fullbeard = Полная борода
hair-facial_longbeard = Длинная борода
hair-facial_vandyke = Ван Дейк
hair-facial_elvis = Элвис
hair-facial_abe = Эйб
hair-facial_chin = Шкиперская
hair-facial_hip = Хипстер
hair-facial_gt = Козлиная
hair-facial_hogan = Хоган
colour-black = чёрный
colour-brown = коричневый
colour-blonde = светлый
colour-red = рыжий
colour-grey = седой
colour-white = белый
colour-blue = синий
colour-green = зелёный
hair-bald = лысый
hair-shaved = бритый
pref-male = Пол: мужской
pref-female = Пол: женский
pref-hair = Волосы: { $what }
pref-hair-colour = Цвет волос: { $what }
pref-facial = Лицо: { $what }
pref-eyes = Глаза: { $what }
pref-darker = Кожа темнее ({ $value })
pref-lighter = Кожа светлее
pref-underwear = Бельё: { $value }
pref-traitor-on = Хочу быть предателем: да
pref-traitor-off = Хочу быть предателем: нет
pref-job-slot = Клик по работе задаёт выбор { $value }
lobby-observe = Наблюдать
lobby-crew = Посмотреть список экипажа
ghost-observing = Вы наблюдаете как призрак. Живые вас не видят и не слышат.
round-mode-extended = Просто веселитесь и отыгрывайте роль!
round-mode-sandbox = Постройте свой корабль: F5 открывает панель песочницы.
sandbox-title = Панель песочницы
sandbox-suit = Надеть снаряжение для космоса
sandbox-kit = { NOUN($item) } ({ $value })
sandbox-object = { NOUN($item) }
sandbox-objects = Любой предмет, страница { $value } из { $pages }:
sandbox-previous = Предыдущая страница
sandbox-next = Следующая страница
sandbox-spawning-on = Песочница: создание предметов включено.
sandbox-spawning-off = Песочница: создание предметов выключено.
vote-choice-sandbox = Песочница
nuke-title = Ядерное устройство
nuke-disk-in = Диск: ++++++++++
nuke-disk-out = Диск: ----------
nuke-time = Время: { $value }
nuke-on = вкл.
nuke-off = выкл.
nuke-timer = Таймер: { $state }
nuke-safety = Предохранитель: { $state }
nuke-anchor = Крепление: { $state }
nuke-display-auth = АУТЕНТИФИКАЦИЯ
nuke-blast = Станция содрогается, и всё поглощает ослепительный свет!
nuke-win-syndicate = Оперативники Синдиката уничтожили станцию!
nuke-killed-everyone = Ядерный взрыв убил всех!
nuke-win-crew = Экипаж спасся с диском: оперативники Синдиката остановлены!
nuke-neutral = Ничья: диск не вывезли, но станция цела. В следующий раз не теряйте диск!
nuke-agent = { $name } был агентом Синдиката.
nuke-agent-dead = { $name } был агентом Синдиката. (мёртв)
nuke-pin-on = Вы включаете целеуказатель. { $deck ->
    [up] Диск на палубе выше.
    [down] Диск на палубе ниже.
   *[level] Диск на этой палубе.
}
nuke-pin-off = Вы выключаете целеуказатель.
nuke-agent-brief = Вы агент Синдиката! Заберите диск и взорвите ядерную бомбу на станции.
nuke-leader-code = Код активации: { $code }
nuke-guide = Как взорвать бомбу. Сначала доставьте её на станцию: щелчок по сложенной бомбе закрепляет её, и сдвинуть её снова можно только через её окно. Затем вставьте диск, наберите код и нажмите E. Выключите предохранитель, установите время и запустите таймер. Теперь её не остановить, вынув диск. R сбрасывает набор. Запомните порядок: диск, код, предохранитель, таймер, диск, бегите.
nuke-radio = Используйте ":t", чтобы говорить по шифрованному каналу Синдиката.
vote-choice-nuclear = Ядерная угроза
pref-nuke-on = Хочу быть ядерным агентом: да
pref-nuke-off = Хочу быть ядерным агентом: нет
rev-not-enough = Недостаточно игроков для революции: этот раунд будет «Расширенным».
rev-leader = Вы один из лидеров революции!
rev-objective-kill = Цель: убить главу отдела { $name }.
rev-objective-capture = Цель: схватить главу отдела { $name }, надев наручники или оглушив.
rev-flash-howto = Вам выдали вспышку: ослепите кого-нибудь, чтобы обратить его.
rev-rp-howto = Щёлкните по человеку правой кнопкой и выберите «Обратить», чтобы позвать его в революцию.
rev-joined = Теперь вы революционер! Помогайте делу и не причиняйте вреда товарищам. Над вашими товарищами красная R, над лидерами синяя. Помогите им свергнуть глав отделов!
rev-brainwashed = Вам промыли мозги! Вы больше не революционер!
rev-remembered = { $name } будто вспоминает, кому на самом деле верен!
rev-already = { $name } уже революционер!
rev-cannot = { $name } не может стать революционером!
rev-wait = Подождите пять секунд, прежде чем спрашивать снова.
rev-asking = Вы пытаетесь обратить: { $name }...
rev-accepted = { $name } присоединяется к революции!
rev-refused = { $name } не поддерживает революцию!
rev-rejected = Вы отвергаете это предательское дело!
rev-question = { $name } спрашивает: хотите присоединиться к революции?
rev-answer-yes = Да!
rev-answer-no = Нет!
rev-reveal = На борту вражеские оперативники: { $names }. Немедленно арестуйте их.
rev-win = Главы отделов свергнуты! Революционеры победили!
rev-lose = Главам отделов удалось остановить революцию!
rev-roll-leaders = Лидеры революции:
rev-roll-converted = Обращённые революционеры:
rev-roll-heads = Главы отделов:
rev-roll-line = { $name } ({ $state })
rev-dead = мёртв
rev-alive = выжил
verb-rev-convert = Обратить
vote-choice-revolution = Революция
vote-choice-rp-revolution = Ролевая революция
ling-power-absorb = Поглотить ДНК
ling-power-transform = Превращение
ling-power-stasis = Регенеративный стазис
ling-power-hive-channel = Канал улья
ling-power-hive-absorb = Поглощение из улья
ling-power-lesser-form = Низшая форма
ling-power-panacea = Анатомическая панацея
ling-power-deaf-sting = Глухое жало
ling-power-blind-sting = Слепое жало
ling-power-silence-sting = Немое жало
ling-power-mimic-voice = Подражание голосу
ling-power-extract-dna = Жало сбора ДНК
ling-power-transformation-sting = Жало превращения
ling-power-paralysis-sting = Парализующее жало
ling-power-death-sting = Смертельное жало
ling-power-boost-range = Дальнее жало
ling-power-epinephrine = Железы адреналина
ling-power-chem-synth = Быстрый синтез
ling-power-adv-chem-synth = Продвинутый синтез
ling-power-engorged-glands = Увеличенные железы
ling-power-digital-camo = Цифровой камуфляж
ling-power-rapid-regen = Быстрая регенерация
ling-title = Генокрад
ling-stats = Химикаты: { $value }/{ $storage }. Очки генома: { $points }. Поглощено геномов: { $absorbed }. Генетический урон: { $damage }.
ling-mimicking = Подражание: { $name }
ling-give-up = Сдаться и покинуть тело
ling-choose = Выберите:
ling-cancel = Отмена
ling-evolve = Развить { $name } ({ $value })
ling-use = { $name } ({ $value })
chat-prompt-mimic = Голос для подражания:
ling-you = Вы генокрад!
ling-howto = F6 открывает способности. Схватите кого-нибудь и используйте «Поглотить ДНК», чтобы забрать геном. Начните строку с :g, чтобы говорить в канале улья.
ling-incapacitated = Мы не в состоянии.
ling-need-chems = Нужно не меньше { $value } единиц химикатов!
ling-genes = Наши геномы ещё собираются. Нужно время, чтобы прийти в себя.
ling-need-grab = Чтобы поглотить существо, нужно держать его в активной руке.
ling-incompatible = Это существо несовместимо с нашей биологией.
ling-ruined = ДНК этого существа безнадёжно испорчена!
ling-busy = Мы уже поглощаем!
ling-absorb-1 = Это существо совместимо. Нужно не двигаться...
ling-absorb-2 = Мы выдвигаем хоботок.
ling-absorb-3 = Мы вонзаем хоботок.
ling-proboscis = { $name } выдвигает хоботок!
ling-stabs = { $name } вонзает хоботок в цель: { $victim }!
ling-stab-pain = Вы чувствуете острую колющую боль!
ling-interrupted = Наше поглощение прервано!
ling-absorbed = Мы поглотили: { $name }!
ling-you-absorbed = Вас поглотил генокрад!
ling-sucks = { $name } высасывает соки из цели: { $victim }!
ling-transforms = Кто-то превращается: { $name }!
ling-genes-cry = Наши гены кричат!
ling-stasis = Мы попытаемся восстановить свою форму.
ling-stasis-failed = Нам не хватает химикатов для регенерации.
ling-regenerated = Мы восстановились.
ling-wakes = { $name } будто встаёт из мёртвых, и все раны зажили.
ling-hive-has = Эта ДНК уже в эфире.
ling-hive-sent = Мы передаём в эфир ДНК: { $name }.
ling-hive-none = В эфире нет новых ДНК.
ling-hive-got = Мы поглощаем из эфира ДНК: { $name }.
ling-hive-say = { $name }: { $text }
ling-cleansed = Мы очищаем своё тело от примесей.
ling-range = Наше горло готовится метнуть жало.
ling-arise = Мы поднимаемся.
ling-camo-on = Мы искажаем свою форму, чтобы ИИ нас не отслеживал.
ling-camo-off = Мы возвращаемся к обычному виду.
ling-voice = Мы меняем голос на: { $name }. На это уходят химикаты, пока мы не прекратим.
ling-voice-back = Мы возвращаем голосовые железы на место.
ling-stung = Мы незаметно жалим: { $name }.
ling-prick = Вы чувствуете лёгкий укол.
ling-ears-pop = В ушах хлопает и громко звенит!
ling-eyes-burn = Глаза ужасно жжёт!
ling-muscles = Мышцы болезненно сводит.
ling-chest = Вы чувствуете лёгкий укол, и грудь стягивает...
ling-sting-useless = Похоже, жало не действует на эту ДНК.
ling-was = Генокрадом был: { $name }.
ling-genomes = Поглощено геномов: { $value }
ling-won = Генокрад справился!
ling-lost = Генокрад провалился!
objective-absorb = Цель №{ $value }: собрать совместимых геномов: { $goal }. { $done ->
    [won] Успех
    [failed] Провал
   *[none] { "" }
}
objective-survive = Цель №{ $value }: остаться в живых до конца. { $done ->
    [won] Успех
    [failed] Провал
   *[none] { "" }
}
vote-choice-changeling = Генокрад
round-mode-meteor = Корабль застрял в сильном метеорном потоке. Нужно бежать с корабля или хотя бы выжить!
blob-alert = Сообщение Центкома: биологическая тревога. На станцию могла попасть опасная форма жизни 5 уровня. Найдите её и сдержите, и не покидайте зону карантина.
blob-confirmed = Подтверждена вспышка биологической угрозы 5 уровня на борту. Весь персонал должен сдержать её.
blob-directive = Биологическая угроза вышла из-под контроля. Центком отдал приказ уничтожить станцию.
blob-staff-won = Персонал победил! Биологическая угроза сдержана.
blob-staff-lost = Персонал проиграл! Станцию уничтожил Центком.
blob-attacks-you = Блоб атакует вас!
blob-weakened = Блоб ослабляет: { $victim }!
blob-knocked-down = Блоб сбивает с ног: { $victim }!
blob-stunned = Блоб оглушает: { $victim }!
vote-choice-meteor = Метеоры
vote-choice-blob = Блоб
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
    [disarm] обезоружить
    [grab] схватить
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
        [zone] Зона удара
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
lattice-built = Вы собираете опорную решётку...
lattice-needs-support = Под обшивку нужна опора.
lattice-sliced = Вы разрезаете стыки решётки...

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
monkey-you-are = Теперь вы — обезьяна.
monkey-won = Обезьяны победили!
monkey-lost = Научный персонал остановил нашествие обезьян!
monkey-was = Обезьяной был: { $name }.
bitten = { $attacker } кусает { $victim }!
bite-missed = { $attacker } пытается укусить { $victim }!
restructuring-announce = ОБЩАЯ ПЕРЕДАЧА ИЗ ГЛАВНОГО ОФИСА: ГОТОВИТСЯ КОРПОРАТИВНАЯ РЕСТРУКТУРИЗАЦИЯ
restructuring-announce-care = Следите за новостями; помните, что мы заботимся о сотрудниках, и все сокращения пройдут деликатно и с сочувствием
restructuring-target = ГЛАВНЫЙ ОФИС: { $name } обвиняется в попытке создать профсоюз и считается угрозой кораблю. Немедленно уволить сотрудника.
restructuring-target-job = ГЛАВНЫЙ ОФИС: { $name } ({ $job }) обвиняется в попытке создать профсоюз и считается угрозой кораблю. Немедленно уволить сотрудника.
restructuring-you = Главный офис распорядился о вашем сокращении. Берегитесь!
vote-choice-monkey = Обезьяны
vote-choice-restructuring = Корпоративная реструктуризация
autotraitor-announce = Это испытательный стенд для бесконечного раунда с предателями. Предатели будут добавляться в раунд автоматически по мере надобности. Возможны ошибки.
traitor-debt = ВНИМАНИЕ: пришло время вернуть долг Синдикату...
traitor-now = Теперь вы предатель.
vote-choice-autotraitor = Автопредатель
spell-magic-missile = Волшебная стрела
spell-disable-tech = Отключение техники
spell-blink = Скачок
spell-teleport = Телепортация
spell-forcewall = Силовая стена
spell-fireball = Огненный шар
spell-knock = Отпирание
spell-invisible-wall = Невидимая стена
spell-blind = Ослепление
spell-mutate = Мутация
spell-blind-eyes = Глаза пронзает боль!
spell-mutate-strong = Вы чувствуете в себе силу!
spell-recharging = { $name }: заклинание ещё не восстановилось.
spell-incapacitated = Не в таком состоянии.
spell-cant-speak = Слова не идут с языка!
spell-no-robe = Без мантии мне не хватает сил.
spell-no-sandals = Без сандалий мне не хватает сил.
spell-no-hat = Без шляпы мне не хватает сил.
spell-learned = Теперь вы знаете заклинание: { $name }.
spell-title = Заклинания
spell-line = { $name } ({ $value }/{ $max })
spell-choose-area = Куда?
spell-choose-target = На кого?
spell-cancel = Отмена
# Вещи волшебника (spells.md)
scroll-learned = Вы выучили заклинание: { $spell }!
orb-see-everything = Вы видите... всё!
orb-stares = { $name } смотрит в шар прорицания, и взгляд стекленеет.
survivor-you = Вы выживший! Своя жизнь дороже всего: никому не верьте и убивайте всех, кто встанет на пути. С оружием в руках самое время свести старые счёты...
book-title = Книга заклинаний
book-uses = Осталось выучить заклинаний: { $value }
book-choose = Какое заклинание выучить? Число после названия — время восстановления в секундах.
book-line = { $name } ({ $value })
book-line-once = { $name } (один раз, на всех)
book-magic-missile = { spell-magic-missile }
book-fireball = { spell-fireball }
book-disable-tech = { spell-disable-tech }
book-forcewall = { spell-forcewall }
book-blink = { spell-blink }
book-teleport = { spell-teleport }
book-knock = { spell-knock }
book-blind = { spell-blind }
book-mutate = { spell-mutate }
book-summon-guns = Призыв оружия
book-armor = Доспех мастера
book-scrying = Шар прорицания
book-desc-magic-missile = Медленные волшебные стрелы во всех вокруг. Каждая сбивает с ног и слегка обжигает.
book-desc-fireball = Огненный шар туда, куда вы смотрите. Мантия не нужна. Осторожнее с теми, кто стоит рядом.
book-desc-disable-tech = Импульс выводит из строя оружие, камеры и почти любую технику вокруг.
book-desc-forcewall = Несокрушимая стена на 30 секунд. Мантия не нужна.
book-desc-blink = Короткий скачок в случайную сторону: чтобы сбежать или, при терпении, пробраться куда нужно.
book-desc-teleport = В выбранное место, в случайную его точку. Выручает в беде, но долго восстанавливается.
book-desc-knock = Открывает двери вокруг. Мантия не нужна.
book-desc-blind = Ненадолго ослепляет одного человека. Мантия не нужна.
book-desc-mutate = Ненадолго вы становитесь халком, видите сквозь стены, не боитесь ни холода, ни огня и не нуждаетесь в воздухе.
book-desc-summon-guns = Вооружает экипаж, который только и ждал повода. Берегитесь перекрёстного огня.
book-desc-armor = Доспех, в котором можно колдовать: защищает лучше и держит космос.
book-desc-scrying = Шар потрескивающего света: загляните в него, чтобы покинуть тело и бродить по кораблю призраком.
book-artefacts = Артефакты
book-artefacts-note = Могущественные вещи, пропитанные магией. Каждая идёт в счёт выучиваемых заклинаний. Лучше оставить их опытным волшебникам.
book-rememorize = Выучить заклинания заново
book-known = Вы уже знаете это заклинание.
book-learned = Вы выучили заклинание: { $spell }.
book-summoned-guns = Вы применили «Призыв оружия».
book-bought-armor = Вы получили доспех волшебника.
book-bought-orb = Вы получили шар прорицания.
book-rememorized = Все заклинания забыты. Теперь можно выучить новые.
book-rememorize-elsewhere = Выучить заклинания заново можно только в святилище волшебника.
admin-no-spell = Нет заклинания «{ $name }». Заклинания: { $text }.
vote-choice-derelict = Заброшенная станция
job-scavenger = Мусорщик

## Генетика (`genetics.md`)
gene-no-breath = Вы перестаёте дышать.
gene-remote-view = Ваш разум расширяется.
gene-regenerate = Вы чувствуете себя странно.
gene-run = Вы чувствуете себя быстрее.
gene-remote-talk = Ваш разум тянется вовне.
gene-morph = Ваша кожа ощущается странно.
gene-blend = Вам одиноко.
gene-hallucination = Ваш разум говорит: «Привет».
gene-no-prints = Ваши пальцы немеют.
gene-shock-immune = Вы чувствуете себя странно.
gene-small = Ваша кожа становится резиновой.
gene-hulk = Ваши мышцы болят.
gene-headache = У вас начинает болеть голова.
gene-fake = Вы чувствуете себя странно.
gene-cough = Вы начинаете кашлять.
gene-clumsy = У вас кружится голова.
gene-twitch = Вы дёргаетесь.
gene-xray = Стены вдруг исчезают.
gene-nervous = Вы нервничаете.
gene-cold-resist = Вашему телу тепло.
gene-blind = Вы ничего не видите.
gene-telekinesis = Вы чувствуете себя умнее.
gene-deaf = Как-то тихо...
gene-hulk-lost = Вы вдруг чувствуете сильную слабость.
gene-seizure = У вас припадок!
gene-seizure-seen = { $name }: начинается припадок!
radiation-weak = Вы чувствуете слабость.
radiation-mutate = Вы мутируете!
injector-trying = { $name } пытается сделать укол ДНК-инъектором: { $victim }!
injector-injected = { $name } делает укол ДНК-инъектором: { $victim }.
injector-you-inject = Вы делаете укол: { $victim }.
genetics-no-occupant = Подходящий пациент не обнаружен.
genetics-transferred = Перенесено.
genetics-replicator-not-ready = Репликатор ещё не готов.
genetics-injector-error = Ошибка при создании инъектора.
genetics-injector-made = Инъектор создан.
genetics-inaprovaline = Инапровалина в крови пациента: { $value } ед.
genetics-occupant = Состояние пациента:
genetics-health = Здоровье, %: { $value } ({ $stat })
genetics-stat-conscious = в сознании
genetics-stat-unconscious = без сознания
genetics-stat-dead = *мёртв*
genetics-radiation = Уровень радиации: { $value }%
genetics-ue = Уникальные ферменты: { $value }
genetics-ui = Уникальный идентификатор: { $value }
genetics-se = Структурные ферменты: { $value }
genetics-ruined = Структура ДНК пациента разрушена до неузнаваемости. Поместите пациента с целой ДНК.
genetics-empty = Сканер пуст.
genetics-no-scanner = Ошибка: ДНК-модификатор не подключён.
genetics-modify-ui = Изменить уникальный идентификатор
genetics-modify-se = Изменить структурные ферменты
genetics-buffers-menu = Буферы: просмотр, правка, перенос
genetics-pulse = Импульс излучения
genetics-emitter-menu = Настройки излучателя
genetics-inject = Ввести инапровалин
genetics-lock = Запереть (не заперт)
genetics-unlock = Отпереть (заперт)
genetics-main-menu = Главное меню
genetics-buffers-link = Меню буферов
genetics-working = Идёт работа... Подождите ({ $value } с)
genetics-duration = Длительность излучения: { $value }
genetics-strength = Интенсивность излучения: { $value }
genetics-duration-label = Длительность
genetics-strength-label = Интенсивность
genetics-block = Выбранный блок: { $value }
genetics-sub = Выбранный подблок: { $value }
genetics-target = Выбранная цель: { $value }
genetics-block-label = Блок
genetics-sub-label = Подблок
genetics-target-label = Цель
genetics-modify-block = Изменить блок:
genetics-irradiate = Облучить
genetics-buffer = Буфер { $value }:
genetics-buffer-empty = Буфер пуст
genetics-buffer-data = Данные: { $value }
genetics-buffer-by = Чьи: { $value }
genetics-label-ui = Метка: уникальный идентификатор
genetics-label-uiue = Метка: уникальный идентификатор и уникальные ферменты
genetics-label-se = Метка: структурные ферменты
genetics-save = Сохранить:
genetics-save-ui = УИ
genetics-save-uiue = УИ+УФ
genetics-save-se = СФ
genetics-transfer-to = Перенести:
genetics-to-occupant = Пациенту
genetics-full-injector = Полный инъектор
genetics-iso-injector = Изолированный инъектор
genetics-iso-pick = Блок для инъектора:
genetics-clear = Очистить буфер

## Teleporters (`teleporters.md`)
tele-locked-in = Координаты зафиксированы
tele-engaged = Телепорт включён!
tele-disengaged = Телепорт выключен!
tele-no-lock = Сбой: координаты не зафиксированы. Восстановите координатную матрицу.
tele-recharging = Ручной телепортер перезаряжается!
tele-row-active = { $value } (активен)
tele-row-inactive = { $value } (неактивен)
tele-row-none = Никуда (опасно)
## Random events (`events.md`)
command-alert = Сообщение NanoTrasen. { $title }: { $text }
alert-anomaly = Аномалия
alert-meteor = Метеоритная угроза
alert-maintenance = Техобслуживание
alert-medical = Медицинская тревога
event-storm-start = Корабль проходит сквозь электрическую бурю. Возможны перебои радиосвязи.
event-storm-end = Корабль вышел из электрической бури. Радиосвязь восстановлена.
event-meteors-start = Корабль входит в метеорный поток.
event-meteors-end = Корабль вышел из метеорного потока.
event-portals-start = Вокруг корабля обнаружены подпространственные возмущения.
event-grid-check = Идёт автоматическая проверка энергосети корабля, ожидайте.
event-radiation-start = Корабль входит в радиационный пояс.
event-radiation-end = Корабль вышел из радиационного пояса.
admin-events-on = { $name } включает случайные события.
admin-events-off = { $name } выключает случайные события.
admin-events-disabled = Случайные события выключены.
admin-event-started = Событие запущено: { $name }.
admin-event-busy = Сейчас уже идёт событие.
admin-no-event = Нет события { $name }. События: { $text }.
## Sinks, showers, toilets and mirrors (`plumbing.md`)
sink-busy = Здесь уже кто-то моется.
sink-start-hands = Вы начинаете мыть руки.
sink-start-item = Вы начинаете мыть { NOUN($item, case: "acc") }.
sink-washed-hands = { $name } моет руки: { NOUN($sink) }.
sink-washed-item = { $name } моет { NOUN($item, case: "acc") } под краном: { NOUN($sink) }.
sink-washed-item-self = Вы моете { NOUN($item, case: "acc") } под краном: { NOUN($sink) }.
sink-fills = { $name } наполняет { NOUN($item, case: "acc") } водой: { NOUN($sink) }.
sink-fills-self = Вы наполняете { NOUN($item, case: "acc") } водой: { NOUN($sink) }.
shower-reads = Вода, похоже, { $temp }.
shower-temp-normal = обычной температуры
shower-temp-freezing = ледяная
shower-temp-boiling = кипяток
shower-valve-start = Вы начинаете поворачивать гаечным ключом вентиль температуры.
shower-adjusted = { $name } настраивает душ гаечным ключом.
shower-adjusted-self = Вы настраиваете душ гаечным ключом.
shower-freezing = Вода ледяная!
shower-searing = Вода обжигает!
toilet-tighter-grip = Нужно держать крепче.
toilet-slam = { $name } бьёт { $victim } об унитаз!
toilet-slam-self = Вы бьёте { $victim } об унитаз!
mirror-smashed = { $name } разбивает зеркало: { NOUN($item, case: "ins") }!
mirror-hit = { $name } бьёт по зеркалу: { NOUN($item, case: "ins") }!

# Устройства (devices.md). Аргументы: $item (id существительного), $value.
device-beep = *бип* *бип*
signaler-open = Теперь радио можно присоединять и настраивать!
signaler-closed = Радио больше нельзя настраивать или присоединять!
igniter-ready = Воспламенитель готов!
igniter-attachable = Теперь воспламенитель можно присоединить!
assembly-secured-timer = Таймер закреплён!
assembly-unsecured-timer = Таймер откреплён!
assembly-secured-prox-igniter = Датчик движения закреплён! Воспламенитель работает!
assembly-unsecured-prox-igniter = Датчик движения откреплён! Воспламенитель не будет работать.
assembly-secured-radio-igniter = Радио закреплено! Воспламенитель работает!
assembly-unsecured-radio-igniter = Радио откреплено! Воспламенитель не будет работать.
assembly-secured-signaler = Сигнализатор закреплён!
assembly-unsecured-signaler = Сигнализатор откреплён!
assembly-secured-prox = Датчик движения закреплён!
assembly-unsecured-prox = Датчик движения откреплён!
bomb-welded = В клапане баллона с плазмой просверлено отверстие. Теперь плазму можно поджечь.
bomb-unwelded = Отверстие заварено.
tvalve-tank-attached = Вы присоединяете баллон к переходному клапану.
tvalve-tank-attached-second = Вы присоединяете баллон к переходному клапану!
tvalve-tanks-full = Уже присоединены два баллона, сначала снимите один!
tvalve-device-attached = Вы присоединяете к управлению клапаном: { NOUN($item) }!
tvalve-device-full = К клапану уже присоединено устройство, сначала снимите его!
tvalve-tank-1 = Первый баллон: присоединён (снять)
tvalve-tank-1-none = Первый баллон: нет
tvalve-tank-2 = Второй баллон: присоединён (снять)
tvalve-tank-2-none = Второй баллон: нет
tvalve-device = Устройство клапана: присоединено (снять)
tvalve-device-none = Устройство клапана: нет
tvalve-open = Клапан: открыт
tvalve-closed = Клапан: закрыт
signaler-send = Отправить сигнал
signaler-frequency = Частота: { $value }
signaler-freq-down-more = Частота - -
signaler-freq-down = Частота -
signaler-freq-up = Частота +
signaler-freq-up-more = Частота + +
signaler-code = Код: { $value }
signaler-code-down-more = Код - -
signaler-code-down = Код -
signaler-code-up = Код +
signaler-code-up-more = Код + +
timer-timing = Идёт отсчёт { $value } (остановить)
timer-not-timing = Отсчёт не идёт { $value } (запустить)
timer-down-more = Время - -
timer-down = Время -
timer-up = Время +
timer-up-more = Время + +
prox-armed = Взведён (датчик движения работает)
prox-unarmed = Не взведён (взвести датчик движения)
