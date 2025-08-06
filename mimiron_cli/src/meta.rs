use anyhow::Result;
use clap::Args;
use mimiron::{
    card_details::Class,
    deck::Format,
    localization::{Locale, Localize},
    meta::meta_deck,
};

#[derive(Args, Clone)]
pub struct MetaArgs {
    class: Class,
    #[arg(default_value = "standard")]
    format: Format,
}

pub fn run(
    args: MetaArgs,
    locale: Locale,
) -> Result<()> {
    let decks = meta_deck(Some(args.class), args.format, locale)?;

    for deck in decks.take(3) {
        println!("{}", deck.in_locale(locale));
    }

    Ok(())
}
