//! Case, number and gender agreement in Russian.

use rs13_i18n::{Args, Lexicon, Localizer, langid};

const NOUNS: &str = r#"{
    "crowbar": (
        gender: masculine,
        sg: (nom: "лом", gen: "лома", dat: "лому", acc: "лом", ins: "ломом", prep: "ломе"),
        pl: Some((nom: "ломы", gen: "ломов", dat: "ломам", acc: "ломы", ins: "ломами", prep: "ломах")),
    ),
    "toolbox": (
        gender: masculine,
        sg: (nom: "ящик", gen: "ящика"),
    ),
    "screwdriver": (
        gender: feminine,
        sg: (nom: "отвёртка", gen: "отвёртки", dat: "отвёртке", acc: "отвёртку", ins: "отвёрткой", prep: "отвёртке"),
    ),
    "wirecutters": (
        gender: plural,
        sg: (nom: "кусачки", gen: "кусачек", dat: "кусачкам", acc: "кусачки", ins: "кусачками", prep: "кусачках"),
    ),
}"#;

const FTL: &str = r#"
attack-with = { $user } { $user_gender ->
        [female] ударила
       *[other] ударил
    } { $target } { NOUN($weapon, case: "ins") }.
item-fell = { NOUN($item) } { GENDER($item) ->
        [feminine] упала
        [neuter] упало
        [plural] упали
       *[masculine] упал
    } на пол.
stack-count = { $count } { $count ->
        [one] { NOUN($item) }
        [few] { NOUN($item, case: "gen") }
       *[many] { NOUN($item, case: "gen", number: "pl") }
    }
look-in = Вы заглядываете в { NOUN($item, case: "acc") }.
"#;

fn ru() -> Localizer {
    Localizer::new(langid!("ru"), &[FTL], Lexicon::from_ron(NOUNS).unwrap()).unwrap()
}

#[test]
fn instrumental_case_and_speaker_gender() {
    let mut args = Args::new();
    args.set("user", "John Smith");
    args.set("user_gender", "male");
    args.set("target", "Bob");
    args.set("weapon", "crowbar");
    assert_eq!(
        ru().format("attack-with", Some(&args)).unwrap(),
        "John Smith ударил Bob ломом."
    );

    args.set("user_gender", "female");
    assert_eq!(
        ru().format("attack-with", Some(&args)).unwrap(),
        "John Smith ударила Bob ломом."
    );
}

#[test]
fn verb_agrees_with_noun_gender() {
    let fell = |item: &str| {
        let mut args = Args::new();
        args.set("item", item.to_owned());
        ru().format("item-fell", Some(&args)).unwrap()
    };
    assert_eq!(fell("crowbar"), "лом упал на пол.");
    assert_eq!(fell("screwdriver"), "отвёртка упала на пол.");
    assert_eq!(fell("wirecutters"), "кусачки упали на пол.");
}

#[test]
fn counted_nouns_use_russian_plural_rules() {
    let count = |n: u32| {
        let mut args = Args::new();
        args.set("item", "crowbar");
        args.set("count", n);
        ru().format("stack-count", Some(&args)).unwrap()
    };
    assert_eq!(count(1), "1 лом");
    assert_eq!(count(3), "3 лома");
    assert_eq!(count(5), "5 ломов");
    assert_eq!(count(11), "11 ломов");
    assert_eq!(count(21), "21 лом");
    assert_eq!(count(22), "22 лома");
}

#[test]
fn missing_case_falls_back_to_nominative_and_is_reported() {
    let mut args = Args::new();
    args.set("item", "toolbox");
    assert_eq!(
        ru().format("look-in", Some(&args)).unwrap(),
        "Вы заглядываете в ящик."
    );

    let missing = Lexicon::from_ron(NOUNS).unwrap().missing_cases();
    assert_eq!(missing.len(), 4);
    assert!(missing.iter().all(|(id, _)| id == "toolbox"));
}

#[test]
fn unknown_noun_is_visible() {
    let mut args = Args::new();
    args.set("item", "no-such-item");
    assert_eq!(
        ru().format("item-fell", Some(&args)).unwrap(),
        "{no-such-item} упал на пол."
    );
}

#[test]
fn unknown_message_is_an_error() {
    assert!(ru().format("does-not-exist", None).is_err());
}

#[test]
fn localizer_is_shareable_across_threads() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Localizer>();
}
