use anyhow::Result;
use clap::{Args, Parser};
use itertools::Itertools;

mod authorization;
mod card;
mod card_details;
mod deck;

#[derive(Parser)]
#[command(author, version)]
pub struct MimironArgs {
    #[command(flatten)]
    mode: MimironModes,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
struct MimironModes {
    /// card text to search for
    #[arg(trailing_var_arg = true)]
    card_name: Option<Vec<String>>, // remmeber to join!

    /// deck code to parse
    #[arg(short, long)]
    deck: Option<String>,

    /// get access token to test API. for development purposes. Should I use #[cfg(debug_assertions) ?
    #[arg(short)]
    token: bool, // remove before release hah !!
}

pub fn run(args: MimironArgs) -> Result<()> {
    let creds = authorization::get_creds_from_env()?;
    let access_token = authorization::get_access_token(creds)?;

    let mode = args.mode;
    if mode.token {
        println!("{access_token}")
    } else if let Some(search_term) = mode.card_name {
        // Card Search
        let res = ureq::get("https://us.api.blizzard.com/hearthstone/cards")
            .query("locale", "en_us")
            .query("textFilter", &search_term.join(" "))
            .query("access_token", &access_token)
            .call()?
            .into_json::<card::CardSearchResponse>()?;

        if res.card_count > 0 {
            let cards = res.cards.into_iter().unique_by(|c| c.name.clone()).take(5);
            for card in cards {
                println!("{card:#}");
            }
        } else {
            println!("No card found. Check your spelling.")
        }
    } else if let Some(deck_string) = mode.deck {
        // Deck Code
        let res = ureq::get("https://us.api.blizzard.com/hearthstone/deck")
            .query("locale", "en_us")
            .query("code", &deck_string)
            .query("access_token", &access_token)
            .call()?
            .into_json::<deck::Deck>()?;

        println!("{res}");
    }

    Ok(())
}
