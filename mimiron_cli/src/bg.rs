use anyhow::Result;
use clap::{ArgGroup, Args};
use mimiron::{
    bg,
    card_details::MinionType,
    localization::{Locale, Localize},
};

#[derive(Args)]
#[command(group = ArgGroup::new("search").required(true).multiple(true))]
pub struct BGArgs {
    /// Text to search for
    #[arg(group = "search")]
    name: Option<String>,

    /// Search by Minion Battlegrounds tier
    #[arg(short, long, group = "search", value_parser = clap::value_parser!(u8).range(1..=7))]
    tier: Option<u8>,

    // Search by Minion type
    #[arg(short = 'T', long = "type", group = "search", value_parser = str::parse::<MinionType>)]
    minion_type: Option<MinionType>,

    /// Include text inside text boxes.
    #[arg(long)]
    text: bool,

    /// Print image links.
    #[arg(short, long)]
    image: bool,
}

pub fn run(args: BGArgs, locale: Locale) -> Result<()> {
    let opts = bg::SearchOptions::empty()
        .with_locale(locale)
        .search_for(args.name)
        .with_tier(args.tier)
        .with_type(args.minion_type)
        .with_text(args.text);
    let cards = bg::lookup(&opts)?;

    for card in cards {
        println!("{:#}", card.in_locale(locale));
        _ = bg::get_and_print_associated_cards(&card, locale);
    }

    Ok(())
}
