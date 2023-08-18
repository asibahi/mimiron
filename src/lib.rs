use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod authorization;
mod bg;
mod card;
mod card_details;
mod deck;
mod prettify;
mod deck_image;

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

pub fn run() -> Result<String> {
    let args = Cli::parse();
    let access_token = authorization::get_access_token().context("failed to get access token.")?;
    match args.command {
        Commands::Card(args) => card::run(args, &access_token),
        Commands::Deck(args) => deck::run(args, &access_token),
        Commands::BG(args) => bg::run(args, &access_token),
        Commands::Token => Ok(access_token),
    }
}
