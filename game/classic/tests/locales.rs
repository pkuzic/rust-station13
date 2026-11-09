//! The shipped locales load, cover the same messages, and Russian nouns are fully declined.

use rs13_classic::{locales, maps};
use rs13_i18n::{Args, Lexicon, Localizer, langid};

fn ru() -> Localizer {
    let lexicon = Lexicon::from_ron(locales::RU_NOUNS).unwrap();
    Localizer::new(langid!("ru"), &[locales::RU_MESSAGES], lexicon).unwrap()
}

fn en() -> Localizer {
    let lexicon = Lexicon::from_ron(locales::EN_NOUNS).unwrap();
    Localizer::new(langid!("en"), &[locales::EN_MESSAGES], lexicon).unwrap()
}

/// Top-level message ids in a Fluent file.
fn message_ids(src: &str) -> Vec<&str> {
    let mut ids: Vec<&str> = src
        .lines()
        .filter(|l| l.starts_with(|c: char| c.is_ascii_alphabetic()))
        .filter_map(|l| l.split_once(" =").map(|(id, _)| id))
        .collect();
    ids.sort_unstable();
    ids
}

#[test]
fn russian_nouns_have_every_case() {
    let missing = Lexicon::from_ron(locales::RU_NOUNS).unwrap().missing_cases();
    assert!(missing.is_empty(), "missing case forms: {missing:?}");
}

#[test]
fn locales_define_the_same_messages() {
    assert_eq!(
        message_ids(locales::EN_MESSAGES),
        message_ids(locales::RU_MESSAGES)
    );
}

#[test]
fn use_on_declines_both_nouns() {
    let mut args = Args::new();
    args.set("user", "Jane Doe");
    args.set("user_gender", "female");
    args.set("item", "crowbar");
    args.set("target", "airlock");
    assert_eq!(
        ru().format("use-on", Some(&args)).unwrap(),
        "Jane Doe открыла шлюз с помощью лома."
    );
    assert_eq!(
        en().format("use-on", Some(&args)).unwrap(),
        "Jane Doe opens the airlock with the crowbar."
    );
}

#[test]
fn speech_frame_is_localised_but_the_words_are_not() {
    let mut args = Args::new();
    args.set("name", "John Smith");
    args.set("verb", "exclaim");
    args.set("text", "Help!");
    assert_eq!(
        en().format("say", Some(&args)).unwrap(),
        "John Smith exclaims, \"Help!\""
    );
    assert_eq!(
        ru().format("say", Some(&args)).unwrap(),
        "John Smith восклицает: «Help!»"
    );
}

#[test]
fn every_message_formats_in_both_locales() {
    let mut args = Args::new();
    args.set("user", "John Smith");
    args.set("user_gender", "male");
    args.set("target", "airlock");
    args.set("weapon", "wrench");
    args.set("item", "screwdriver");
    args.set("count", 2);
    args.set("name", "John Smith");
    args.set("verb", "ask");
    args.set("act", "scratch");
    args.set("name", "Someone");
    args.set("done", "won");
    args.set("kind", "restart");
    args.set("what", "something");
    args.set("pages", 2);
    args.set("storage", 50);
    args.set("points", 5);
    args.set("absorbed", 1);
    args.set("damage", 30);
    args.set("goal", 4);
    args.set("names", "Someone, Someone Else");
    args.set("code", "12345");
    args.set("deck", "up");
    args.set("state", "on");
    args.set("key", "abc");
    args.set("verb", "kick");
    args.set("by", "Someone");
    args.set("address", "127.0.0.1");
    args.set("cost", 2);
    args.set("made", 4);
    args.set("a", "bruised");
    args.set("b", "numb");
    args.set("text", "something");
    args.set("choice", "Yes");
    args.set("mode", "extended");
    args.set("ready", 5);
    args.set("players", 5);
    args.set("status", "None");
    args.set("job", "engineer");
    args.set("burn", 5);
    args.set("metal", 5);
    args.set("glass", 5);
    args.set("state", "conscious");
    args.set("oxy", 5);
    args.set("tox", 5);
    args.set("reagent", "Dexalin");
    args.set("max", 50);
    args.set("minutes", 1);
    args.set("seconds", 30);
    args.set("title", "lord");
    args.set("villain", "jeff");
    args.set("enemy", "Lord Jeff");
    args.set("cost", 2);
    args.set("hp", 30);
    args.set("mp", 10);
    args.set("enemyhp", 45);
    args.set("text", "Hello?");
    args.set("attacker", "Alice Abbott");
    args.set("victim", "Bob Barnes");
    args.set("zone", "zone-l_arm");
    args.set("value", 75);
    args.set("intent", "harm");
    args.set("equipment", "on");
    args.set("lighting", "off");
    args.set("environment", "on");
    args.set("slot", "mask");
    args.set("what", "drop");
    args.set("colour", "dark-red");
    args.set("area", "Engineering");
    args.set("oxy", 0);
    args.set("tox", 0);
    args.set("burn", 3);
    args.set("brute", 12);
    args.set("health", 85);
    args.set("dead", "no");
    args.set("celsius", 37);
    args.set("fahrenheit", 99);
    args.set("on", "yes");
    args.set("open", "no");
    args.set("cell", "meter");
    args.set("watts", 5000);
    args.set("rating", 2500);
    args.set("charge", 1000);
    args.set("frequency", "145.9");
    args.set("auto", "yes");
    args.set("load", 120);
    args.set("mode", 3);
    args.set("state", 1);
    args.set("supply", 30000);
    args.set("hot", 900);
    args.set("cold", 100);
    args.set("hotkpa", 4000);
    args.set("coldkpa", 3000);
    args.set("temperature", 120.5);
    args.set("pressure", 4000.0);
    args.set("charging", "yes");
    args.set("bin", "disposal-bin");
    args.set("cabinet", "extinguisher-cabinet");
    args.set("seat", "bed");
    args.set("connected", "yes");
    args.set("verb", "chew");
    args.set("tool", "lighter");
    args.set("level", "half");
    args.set("out", "no");
    args.set("first", "crowbar");
    args.set("second", "breath-mask");
    args.set("third", "emergency-oxygen");
    args.set("spell", "Fireball");
    args.set("stat", "Conscious");
    args.set("temp", "normal");
    args.set("sink", "sink");
    args.set("new", "Ann Bell");
    args.set("procedure", "Appendectomy");
    for (loc, src) in [(ru(), locales::RU_MESSAGES), (en(), locales::EN_MESSAGES)] {
        for id in message_ids(src) {
            let text = loc.format(id, Some(&args)).unwrap();
            assert!(!text.contains('{'), "{id}: unresolved noun in {text:?}");
        }
    }
}

#[test]
fn attack_message_declines_item_and_body_part() {
    let mut args = Args::new();
    args.set("attacker", "Alice Abbott");
    args.set("victim", "Bob Barnes");
    args.set("item", "crowbar");
    args.set("zone", "zone-l_arm");
    assert_eq!(
        ru().format("attacked-with", Some(&args)).unwrap(),
        "Alice Abbott бьёт Bob Barnes ломом в левую руку!"
    );
    assert_eq!(
        en().format("attacked-with", Some(&args)).unwrap(),
        "Bob Barnes has been attacked in the left arm with the crowbar by Alice Abbott!"
    );
}

#[test]
fn every_area_of_every_map_has_a_name() {
    for map in [maps::ENGINEERING_CHAMBER, maps::LUNA_ENGINEERING] {
        let map = rs13_world::TileMap::from_ron(map, 0).unwrap();
        for area in &map.area_names {
            for loc in [en(), ru()] {
                let id = format!("area-{area}");
                assert!(loc.format(&id, None).is_ok(), "{id} is missing");
            }
        }
    }
}

/// The sandbox panel lists every item by name: each must have a noun in both languages.
#[test]
fn every_item_can_be_named_on_the_sandbox_panel() {
    let protos = rs13_classic::objects::ItemProtos::default();
    for locale in ["en", "ru"] {
        let lexicon = std::fs::read_to_string(format!(
            "{}/locales/{locale}/nouns.ron",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        for id in protos.0.keys() {
            assert!(
                lexicon.contains(&format!("\"{id}\"")),
                "{locale} has no noun for {id}"
            );
        }
    }
}
