//! The classic SS13 game build, modelled on NSV Luna.
//!
//! Luna is a behaviour reference only: see `docs/GAME_REFERENCE.md`. No DM code is
//! translated into this crate.

/// The NSV Luna commit (github.com/SS13/luna) that defines reference behaviour.
pub const REFERENCE_COMMIT: &str = "e13cf12";

/// Localisation files shipped with the classic build.
pub mod locales {
    pub const EN_MESSAGES: &str = include_str!("../locales/en/messages.ftl");
    pub const EN_NOUNS: &str = include_str!("../locales/en/nouns.ron");
    pub const RU_MESSAGES: &str = include_str!("../locales/ru/messages.ftl");
    pub const RU_NOUNS: &str = include_str!("../locales/ru/nouns.ron");
}
