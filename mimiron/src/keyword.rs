use crate::{
        card_details::{LocalizedName, METADATA},
        localization::{Locale, Localize},
};
use anyhow::Result;
use serde::Deserialize;
use std::fmt::Display;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Keyword {
    name: LocalizedName,
    ref_text: LocalizedName,
}
impl Keyword {
    #[must_use]
    pub fn contains(&self, search_term: &str) -> bool {
        self.name.contains(search_term)
    }
    #[must_use]
    pub fn name(&self, locale: Locale) -> String {
        self.name.in_locale(locale).to_string()
    }
    #[must_use]
    pub fn text(&self, locale: Locale) -> String {
        self.ref_text.in_locale(locale).to_string()
    }
}
impl Localize for Keyword {
    fn in_locale(&self, locale: Locale) -> impl Display {
        format!("{}\n\t{}", self.name(locale), self.text(locale))
    }
}

pub fn lookup(search_term: &str) -> Result<impl Iterator<Item = Keyword> + '_> {
    let res = METADATA.keywords.iter().filter(|kw| kw.contains(search_term)).cloned();

    Ok(res)
}
