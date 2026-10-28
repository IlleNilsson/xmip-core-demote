//! Demote, the standard unit that writes a context value into the content
//! at send, as data (ADR-0066, amendment 2026-10-22).

use node::Stage;
use node::unit::Declaration;
use xcore::settings::{Applies, Kind, Presence, Setting, Settings};

/// Demote's name, as a step names it.
pub const DEMOTE: &str = "Demote";

/// Demote, as data: at send alone.
pub const DECLARATION: &Declaration = &Declaration {
    name: DEMOTE,
    capability: "demote",
    stages: &[Stage::Send],
    settings: &Settings {
        technology: DEMOTE,
        settings: &[
            Setting {
                name: "property",
                kind: Kind::Text,
                presence: Presence::Required,
                meaning: "The context property whose value is demoted into the content.",
                applies: Applies::Both,
            },
            Setting {
                name: "language",
                kind: Kind::Text,
                presence: Presence::Required,
                meaning: "The path language the value is written with.",
                applies: Applies::Both,
            },
            Setting {
                name: "expression",
                kind: Kind::Text,
                presence: Presence::Required,
                meaning: "The path the value is written to, in its language.",
                applies: Applies::Both,
            },
        ],
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demote_is_declared_soundly() {
        assert!(
            DECLARATION.problems().is_empty(),
            "{:?}",
            DECLARATION.problems()
        );
    }
}
