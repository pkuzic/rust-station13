//! Localisation built on Fluent, with grammatical case, number and gender for nouns.
//!
//! Every in-game noun (item, mob, job, machine) is defined once per locale in a noun
//! lexicon, keyed by its prototype id. Fluent messages then ask for the form they need:
//!
//! ```ftl
//! attack-with = { $user } ударил { $target } { NOUN($weapon, case: "ins") }.
//! item-fell = { NOUN($item) } { GENDER($item) ->
//!     [feminine] упала
//!    *[other] упал
//! } на пол.
//! ```
//!
//! `NOUN($id, case: "...", number: "...")` returns the declined form, falling back to
//! the nominative singular. `GENDER($id)` returns `masculine`, `feminine`, `neuter` or
//! `plural` for use in selectors. Player and character names are English-only and are
//! passed as plain strings; they are never declined.

use std::collections::HashMap;
use std::sync::Arc;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use serde::Deserialize;
use unic_langid::LanguageIdentifier;

pub use fluent_bundle::FluentArgs as Args;
pub use unic_langid::langid;

/// Grammatical case. English nouns only need the nominative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Case {
    Nom,
    Gen,
    Dat,
    Acc,
    Ins,
    Prep,
}

impl Case {
    pub const ALL: [Case; 6] = [Case::Nom, Case::Gen, Case::Dat, Case::Acc, Case::Ins, Case::Prep];

    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "nom" => Case::Nom,
            "gen" => Case::Gen,
            "dat" => Case::Dat,
            "acc" => Case::Acc,
            "ins" => Case::Ins,
            "prep" => Case::Prep,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    Masculine,
    Feminine,
    Neuter,
    /// Pluralia tantum, e.g. "ножницы".
    Plural,
}

impl Gender {
    pub fn as_str(self) -> &'static str {
        match self {
            Gender::Masculine => "masculine",
            Gender::Feminine => "feminine",
            Gender::Neuter => "neuter",
            Gender::Plural => "plural",
        }
    }
}

/// The case forms of a noun in one grammatical number.
#[derive(Debug, Clone, Deserialize)]
pub struct Forms {
    pub nom: String,
    #[serde(default)]
    pub r#gen: Option<String>,
    #[serde(default)]
    pub dat: Option<String>,
    #[serde(default)]
    pub acc: Option<String>,
    #[serde(default)]
    pub ins: Option<String>,
    #[serde(default)]
    pub prep: Option<String>,
}

impl Forms {
    fn get(&self, case: Case) -> Option<&str> {
        match case {
            Case::Nom => Some(&self.nom),
            Case::Gen => self.r#gen.as_deref(),
            Case::Dat => self.dat.as_deref(),
            Case::Acc => self.acc.as_deref(),
            Case::Ins => self.ins.as_deref(),
            Case::Prep => self.prep.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Noun {
    pub gender: Gender,
    /// Singular forms.
    pub sg: Forms,
    /// Plural forms; absent for nouns that are never counted.
    #[serde(default)]
    pub pl: Option<Forms>,
}

impl Noun {
    /// The requested form, falling back to the nominative of the same number, then to
    /// the nominative singular.
    pub fn form(&self, case: Case, plural: bool) -> &str {
        let forms = match (&self.pl, plural) {
            (Some(pl), true) => pl,
            _ => &self.sg,
        };
        forms.get(case).unwrap_or(&forms.nom)
    }
}

/// All nouns of one locale, keyed by prototype id.
#[derive(Debug, Clone, Default)]
pub struct Lexicon(HashMap<String, Noun>);

impl Lexicon {
    pub fn from_ron(text: &str) -> Result<Self, I18nError> {
        // Implicit `Some` lets case forms be written as plain strings.
        ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
            .from_str(text)
            .map(Lexicon)
            .map_err(|e| I18nError::Lexicon(e.to_string()))
    }

    pub fn get(&self, id: &str) -> Option<&Noun> {
        self.0.get(id)
    }

    /// For locales with grammatical case: report every noun missing a case form.
    /// Returns `(noun id, description)` pairs, sorted by id.
    pub fn missing_cases(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (id, noun) in &self.0 {
            for (number, forms) in [("sg", Some(&noun.sg)), ("pl", noun.pl.as_ref())] {
                let Some(forms) = forms else { continue };
                for case in Case::ALL {
                    if forms.get(case).is_none() {
                        out.push((id.clone(), format!("{number} {case:?}")));
                    }
                }
            }
        }
        out.sort();
        out
    }
}

#[derive(Debug, thiserror::Error)]
pub enum I18nError {
    #[error("invalid noun lexicon: {0}")]
    Lexicon(String),
    #[error("invalid Fluent resource: {0}")]
    Resource(String),
    #[error("unknown message `{0}`")]
    UnknownMessage(String),
    #[error("error formatting `{id}`: {errors}")]
    Format { id: String, errors: String },
}

/// Formats messages for one locale. `Send + Sync`, so the server can share one per locale.
pub struct Localizer {
    bundle: FluentBundle<FluentResource>,
}

impl Localizer {
    pub fn new(
        locale: LanguageIdentifier,
        ftl_sources: &[&str],
        lexicon: Lexicon,
    ) -> Result<Self, I18nError> {
        let mut bundle = FluentBundle::new_concurrent(vec![locale]);
        // Unicode isolation marks are for mixed-direction text and only add noise here.
        bundle.set_use_isolating(false);

        for src in ftl_sources {
            let res = FluentResource::try_new((*src).to_owned())
                .map_err(|(_, errs)| I18nError::Resource(format!("{errs:?}")))?;
            bundle
                .add_resource(res)
                .map_err(|errs| I18nError::Resource(format!("{errs:?}")))?;
        }

        let lexicon = Arc::new(lexicon);
        let nouns = Arc::clone(&lexicon);
        bundle
            .add_function("NOUN", move |positional, named| {
                let Some(FluentValue::String(id)) = positional.first() else {
                    return FluentValue::Error;
                };
                let Some(noun) = nouns.get(id) else {
                    // Visible in-game so missing translations get noticed and reported.
                    return FluentValue::String(format!("{{{id}}}").into());
                };
                let case = match named.get("case") {
                    Some(FluentValue::String(c)) => Case::parse(c).unwrap_or(Case::Nom),
                    _ => Case::Nom,
                };
                let plural = matches!(named.get("number"), Some(FluentValue::String(n)) if n == "pl");
                FluentValue::String(noun.form(case, plural).to_owned().into())
            })
            .map_err(|e| I18nError::Resource(e.to_string()))?;

        let genders = Arc::clone(&lexicon);
        bundle
            .add_function("GENDER", move |positional, _named| match positional.first() {
                Some(FluentValue::String(id)) => match genders.get(id) {
                    Some(noun) => FluentValue::String(noun.gender.as_str().into()),
                    None => FluentValue::String("masculine".into()),
                },
                _ => FluentValue::Error,
            })
            .map_err(|e| I18nError::Resource(e.to_string()))?;

        Ok(Self { bundle })
    }

    pub fn format(&self, id: &str, args: Option<&FluentArgs>) -> Result<String, I18nError> {
        let msg = self
            .bundle
            .get_message(id)
            .ok_or_else(|| I18nError::UnknownMessage(id.to_owned()))?;
        let pattern = msg
            .value()
            .ok_or_else(|| I18nError::UnknownMessage(id.to_owned()))?;
        let mut errors = Vec::new();
        let text = self.bundle.format_pattern(pattern, args, &mut errors);
        if errors.is_empty() {
            Ok(text.into_owned())
        } else {
            Err(I18nError::Format {
                id: id.to_owned(),
                errors: format!("{errors:?}"),
            })
        }
    }
}
