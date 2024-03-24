use crate::{
    card_details::Class,
    deck::{lookup, Deck, Format, LookupOptions},
    localization::{Locale, Localize},
    AGENT,
};
use anyhow::{anyhow, Result};

// Deck suggestion look up using d0nkey's site.
// For personal use only unless got permission from d0nkey.

#[allow(clippy::needless_pass_by_value)]
pub fn meta_deck(class: Class, format: Format, locale: Locale) -> Result<Deck> {
    // Standard Demon Hunter Deck.
    // https://www.d0nkey.top/decks?format=2&player_class=DEMONHUNTER

    let class = class.in_en_us().to_string().to_ascii_uppercase().replace(' ', "");
    let fmt_num = match format {
        Format::Standard => "2",
        Format::Wild => "1",
        Format::Twist => "4",
        _ => anyhow::bail!("Format not supported on d0nkey"),
    };

    let req = AGENT
        .get("https://www.d0nkey.top/decks")
        .query("format", fmt_num)
        .query("player_class", &class);

    let fst_try = req.clone().call()?.into_string()?;

    let fst_try =
        fst_try.split_ascii_whitespace().find(|l| l.starts_with("AA") && !l.contains("span"));

    let code = match fst_try {
        Some(code) => code.to_string(),
        None => req
            .query("rank", "all")
            .call()?
            .into_string()?
            .split_ascii_whitespace()
            .find(|l| l.starts_with("AA") && !l.contains("span"))
            .ok_or(anyhow!("No deck found for given class and format."))?
            .to_string(),
    };

    let opts = LookupOptions::lookup(code).with_locale(locale);

    lookup(&opts)
}
