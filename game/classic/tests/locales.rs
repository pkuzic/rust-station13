//! The shipped locales load, cover the same messages, and Russian nouns are fully declined.

use rs13_classic::locales;
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
