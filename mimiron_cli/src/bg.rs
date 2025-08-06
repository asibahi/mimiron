use anyhow::Result;
use clap::{ArgGroup, Args};
use mimiron::{
    bg,
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

    /// Search by Minion type
    #[arg(short = 'T', long = "type", group = "search")]
    minion_type: Option<String>,

    /// Include text inside text boxes.
    #[arg(long)]
    text: bool,

    /// Print image links.
    #[arg(short, long)]
    image: bool,

    #[arg(long, hide = true)]
    debug: bool,
}

pub fn run(
    args: BGArgs,
    locale: Locale,
) -> Result<()> {
    let opts = bg::SearchOptions::empty()
        .with_locale(locale)
        .search_for(args.name.as_deref())
        .with_tier(args.tier)
        .with_type(
            args.minion_type
                .and_then(|s| s.parse().inspect_err(|e| eprintln!("{e}")).ok()),
        )
        .with_text(args.text)
        .debug(args.debug);

    let cards = bg::lookup(opts)?;

    for card in cards {
        println!("{:#}", card.in_locale(locale));
        if args.image {
            println!("\tImage: {}", card.image);
        }
        for (card, assoc) in bg::get_associated_cards(&card, locale, true) {
            bg::print_assoc_card(&card, locale, assoc);
        }
    }

    Ok(())
}
