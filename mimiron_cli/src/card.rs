use anyhow::Result;
use clap::Args;
use mimiron::{
    card,
    localization::{Locale, Localize},
};

#[allow(clippy::struct_excessive_bools)]
#[derive(Args)]
pub struct CardArgs {
    /// Text to search for
    name: String,

    /// Include text inside text boxes and flavor text
    #[arg(short, long)]
    text: bool,

    /// Include reprints
    #[arg(short, long)]
    reprints: bool,

    /// Include non-collectible cards. Expect weird output.
    #[arg(short, long)]
    all: bool,

    /// Print image links
    #[arg(short, long)]
    image: bool,

    #[arg(long, hide = true)]
    debug: bool,
}

pub fn run(args: CardArgs, locale: Locale) -> Result<()> {
    let opts = card::SearchOptions::search_for(&args.name)
        .with_locale(locale)
        .with_text(args.text)
        .include_reprints(args.reprints)
        .include_noncollectibles(args.all)
        .debug(args.debug);

    let cards = card::lookup(opts)?.take(30);

    for card in cards {
        println!("{:#}", card.in_locale(locale));
        if args.image {
            println!("\tImage: {}", card.image);
        }
    }

    Ok(())
}
