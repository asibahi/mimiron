use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod authorization;
mod card;
mod card_details;
mod deck;

#[derive(Parser)]
#[command(author, version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Search for a constructed card by name
    ///
    /// Make sure the card's name is surrounded by quotation marks if it includes spaces or non-letter characters.
    /// For example, "Al'Akir" needs to be surrounded by quotation marks. So does "Ace Hunter".
    Card(card::CardArgs),

    /// Get the cards in a deck code.
    ///
    /// Deck code must be _only_ the deck code. The long code you get straight from Hearthstone's copy deck button is not usable.
    Deck(deck::DeckArgs),

    #[clap(hide = true)]
    Token,
}

pub fn run() -> Result<()> {
    let args = Cli::parse();
    let access_token = authorization::get_access_token().context("failed to get access token.")?;
    match args.command {
        Commands::Card(args) => card::run(args, &access_token),
        Commands::Deck(args) => deck::run(args, &access_token),
        Commands::Token => {
            println!("{}", access_token);
            Ok(())
        }
    }
}
