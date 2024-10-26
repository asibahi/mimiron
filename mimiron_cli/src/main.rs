use anyhow::Result;
use clap::{Parser, Subcommand};
use mimiron::localization::{Locale, Localize};

mod bg;
mod card;
mod deck;
mod meta;

#[derive(Parser)]
#[command(author, version)]
struct Cli {
    #[arg(short, long, global = true, default_value("enUS"), value_parser(str::parse::<Locale>))]
    locale: Locale,

    #[arg(env("BLIZZARD_CLIENT__ID"), hide_env_values(true))]
    id: String,

    #[arg(env("BLIZZARD_CLIENT__SECRET"), hide_env_values(true))]
    secret: String,

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
    #[command(alias("keyword"))]
    KW { input: String },
}

pub fn run() -> Result<()> {
    let args = Cli::parse();
    let locale = args.locale;

    mimiron::set_blizzard_client_auth(args.id, args.secret);

    match args.command {
        Commands::Card(args) => card::run(args, locale)?,
        Commands::Deck(args) => deck::run(args, locale)?,
        Commands::BG(args) => bg::run(args, locale)?,
        Commands::Meta(args) => meta::run(args, locale)?,

        Commands::Token => println!("{}", mimiron::get_access_token()),
        Commands::KW { input } => mimiron::keyword::lookup(&input)?
            .for_each(|kw| println!("{}", kw.in_locale(locale))),
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Encountered error: {e}");
        std::process::exit(1)
    }
}
