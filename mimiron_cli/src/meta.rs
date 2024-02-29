use anyhow::Result;
use clap::Args;
use mimiron::{
    card_details::Class,
    deck::{meta_deck, Format},
    localization::{Locale, Localize},
};

#[derive(Args, Clone)]
pub struct MetaArgs {
    class: Class,
    #[arg(default_value = "standard")]
    format: Format,
}

pub(crate) fn run(args: MetaArgs, locale: Locale) -> Result<()> {
    let deck = meta_deck(args.class, args.format, locale)?;

    println!("{}", deck.in_locale(locale));

    Ok(())
}
