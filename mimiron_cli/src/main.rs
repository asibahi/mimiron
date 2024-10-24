use anyhow::Result;
use clap::{Parser, Subcommand};
use mimiron::localization::Locale;

mod bg;
mod card;
mod deck;
mod keyword;
mod meta;

#[derive(Parser)]
#[command(author, version)]
struct Cli {
    #[arg(short, long, global = true, default_value("enUS"), value_parser(str::parse::<Locale>))]
    locale: Locale,

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

    // Get a meta deck
    #[clap(hide = true)]
    Meta(meta::MetaArgs),

    #[clap(hide = true)]
    KW(keyword::KewordArgs),
}

pub fn run() -> Result<()> {
    let args = Cli::parse();
    let locale = args.locale;

    match args.command {
        Commands::Card(args) => card::run(args, locale)?,
        Commands::Deck(args) => deck::run(args, locale)?,
        Commands::BG(args) => bg::run(args, locale)?,
        Commands::Token => println!("{}", mimiron::get_access_token()),
        Commands::Meta(args) => meta::run(args, locale)?,
        Commands::KW(args) => keyword::run(args, locale)?,
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Encountered error: {e}");
        std::process::exit(1)
    }
}
