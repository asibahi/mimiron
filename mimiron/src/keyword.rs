use crate::{
    card_details::{get_metadata, LocalizedName},
    localization::{Locale, Localize},
    CardTextDisplay,
};
use anyhow::Result;
use compact_str::{CompactString, ToCompactString};
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
    pub fn name(&self, locale: Locale) -> CompactString {
        self.name.in_locale(locale).to_compact_string()
    }
    #[must_use]
    pub fn text(&self, locale: Locale) -> CompactString {
        self.ref_text.in_locale(locale).to_compact_string()
    }
}
impl Localize for Keyword {
    fn in_locale(&self, locale: Locale) -> impl Display {
        format!("{}\n{}", self.name(locale), self.text(locale).to_console())
    }
}

pub fn lookup(search_term: &str) -> Result<impl Iterator<Item = Keyword> + '_> {
    let mut res = get_metadata()
        .keywords
        .clone()
        .into_iter()
        .filter(|kw| kw.contains(search_term))
        .peekable();

    anyhow::ensure!(res.peek().is_some(), "No keyword found with name \"{search_term}\".",);

    Ok(res)
}
