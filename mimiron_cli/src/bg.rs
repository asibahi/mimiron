use anyhow::Result;
use clap::{ArgGroup, Args};
use std::str::FromStr;

use mimiron::{bg, card_details::MinionType, ApiHandle};

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
    #[arg(short = 'T', long = "type", group = "search", value_parser = MinionType::from_str)]
    minion_type: Option<MinionType>,

    /// Include text inside text boxes.
    #[arg(long)]
    text: bool,

    /// Print image links.
    #[arg(short, long)]
    image: bool,
}

pub(crate) fn run(args: BGArgs, api: &ApiHandle) -> Result<()> {
    let opts = bg::SearchOptions::new(args.name, args.tier, args.minion_type, args.text);
    let cards = bg::get(&opts, api)?;

    for card in cards {
        println!("{card:#}");
        bg::get_and_print_associated_cards(card, api);
    }

    Ok(())
}
