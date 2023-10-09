use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod authorization;
mod bg;
mod card;
mod card_details;
mod deck;
mod deck_image;
mod helpers;

#[derive(Parser)]
#[command(author, version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for a constructed card by name
    ///
    /// Make sure the card's name is surrounded by quotation marks if it includes spaces or non-letter characters.
    /// For example, "Al'Akir" needs to be surrounded by quotation marks. So does "Ace Hunter".
    Card(card::CardArgs),

    /// Get the cards in a deck code. Or compare two decks.
    ///
    /// Deck codes must be _only_ the deck code. The long code you get straight from Hearthstone's copy deck button is not usable.
    Deck(deck::DeckArgs),

    /// Search for a Battlegrounds card by name
    ///
    /// Make sure the card's name is surrounded by quotation marks if it includes spaces or non-letter characters.
    /// For example, "Al'Akir" needs to be surrounded by quotation marks. So does "The Rat King".
    BG(bg::BGArgs),

    #[clap(hide = true)]
    Token,
}

pub struct Api {
    agent: ureq::Agent,
    access_token: String,
}

pub fn run_cli() -> Result<()> {
    let args = Cli::parse();

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(2))
        .user_agent("mimiron cli https://github.com/asibahi/mimiron")
        .build();

    let access_token =
        authorization::get_access_token(&agent).with_context(|| "failed to get access token.")?;

    let api = Api {
        agent,
        access_token,
    };

    match args.command {
        Commands::Card(args) => card::run(args, &api)?,
        Commands::Deck(args) => deck::run(args, &api)?,
        Commands::BG(args) => bg::run(args, &api)?,
        Commands::Token => println!("{}", api.access_token),
    }

    Ok(())
}
