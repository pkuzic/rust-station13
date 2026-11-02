//! Navigation beacons (Luna's `navbeacon`): transponders under the floor that tell bots where
//! they are and where to go next. There are no bots yet, so they only wait with what the map
//! set. Spec: `docs/reference/navbeacons.md`.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, GridPos, NetId, ObjectSpec, TilePos};

pub const NAVBEACON: &str = "navbeacon";
/// Luna's default frequency: security's patrol routes. Delivery beacons set their own.
pub const DEFAULT_FREQUENCY: u16 = 1445;

/// What a beacon answers with.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct NavBeacon {
    pub frequency: u16,
    pub location: String,
    pub codes: BTreeMap<String, String>,
}

/// Luna `set_codes`: `"patrol;next_patrol=Chapel"`, where a code with no value counts as "1".
pub fn parse_codes(text: &str) -> BTreeMap<String, String> {
    text.split(';')
        .filter(|entry| !entry.is_empty())
        .map(|entry| match entry.split_once('=') {
            Some((key, value)) => (key.to_owned(), value.to_owned()),
            None => (entry.to_owned(), "1".to_owned()),
        })
        .collect()
}

pub fn spawn_navbeacon(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let var = |name: &str| {
        spec.vars
            .get(name)
            .map(|v| v.trim().trim_matches('"').to_owned())
            .unwrap_or_default()
    };
    let beacon = NavBeacon {
        frequency: var("freq").parse().unwrap_or(DEFAULT_FREQUENCY),
        location: var("location"),
        codes: parse_codes(&var("codes_txt")),
    };
    world
        .spawn((GridPos(pos), id, Appearance::new(NAVBEACON, "navbeacon0"), beacon))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_read_as_lunas_do() {
        let codes = parse_codes("patrol;next_patrol=Chapel");
        assert_eq!(codes["patrol"], "1");
        assert_eq!(codes["next_patrol"], "Chapel");
        assert!(parse_codes("").is_empty());
    }
}
