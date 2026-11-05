//! Object verbs (BYOND's right-click verbs, Luna's `verb/` procs): what the right-click
//! menu offers for each thing beyond Examine, Use and Pull. The client lists them by
//! sprite; the server checks and does them. Spec: `docs/reference/hud.md`, Right-click menu.

/// Open or close a closet or crate (Luna `closet/verb_toggleopen`), within a tile.
pub const TOGGLE_OPEN: &str = "toggle-open";
/// Lock or unlock a secure locker with the access for it (Luna
/// `secure_closet/verb_togglelock`), from beside it, while shut.
pub const TOGGLE_LOCK: &str = "toggle-lock";
/// Turn a chair a quarter anticlockwise, and whoever sits in it (Luna `chair/rotate`),
/// within a tile.
pub const ROTATE_CHAIR: &str = "rotate-chair";
/// Switch a flashlight or lamp you carry (Luna `flashlight/verb/toggle`).
pub const TOGGLE_LIGHT: &str = "toggle-light";
/// Switch a jetpack you carry (Luna `jetpack/verb/toggle`).
pub const TOGGLE_JETPACK: &str = "toggle-jetpack";

/// Ask someone to join an RP revolution (Luna's "Rev-Convert"). The client only offers it to those
/// in on one, and the server checks.
pub const REV_CONVERT: &str = "rev-convert";

/// Every verb, for turning a name a client sent into one of ours.
pub const ALL: [&str; 6] = [
    TOGGLE_OPEN,
    TOGGLE_LOCK,
    ROTATE_CHAIR,
    TOGGLE_LIGHT,
    TOGGLE_JETPACK,
    REV_CONVERT,
];

/// The verb with this name, if there is one.
pub fn named(name: &str) -> Option<&'static str> {
    ALL.into_iter().find(|v| *v == name)
}

/// The verbs a thing with this sprite offers, in menu order.
pub fn verbs_for(sprite: &str) -> Vec<&'static str> {
    let mut verbs = Vec::new();
    if crate::closets::is_closet(sprite) {
        verbs.push(TOGGLE_OPEN);
        if sprite.starts_with("locker-") {
            verbs.push(TOGGLE_LOCK);
        }
    }
    if crate::chairs::is_chair(sprite) {
        verbs.push(ROTATE_CHAIR);
    }
    if crate::flashlights::kind(sprite).is_some() {
        verbs.push(TOGGLE_LIGHT);
    }
    if crate::jetpacks::is_jetpack(sprite) {
        verbs.push(TOGGLE_JETPACK);
    }
    verbs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn things_offer_luna_verbs() {
        assert_eq!(verbs_for("closet"), [TOGGLE_OPEN]);
        assert_eq!(verbs_for("locker-engineer"), [TOGGLE_OPEN, TOGGLE_LOCK]);
        assert_eq!(verbs_for("chair"), [ROTATE_CHAIR]);
        assert_eq!(verbs_for("flashlight"), [TOGGLE_LIGHT]);
        assert!(verbs_for("wrench").is_empty());
    }
}
