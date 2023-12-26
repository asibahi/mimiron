use anyhow::Result;
use clap::Args;
use mimiron::card;

#[allow(clippy::struct_excessive_bools)]
#[derive(Args)]
pub(crate) struct CardArgs {
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
}

pub(crate) fn run(args: CardArgs) -> Result<()> {
    let opts = card::SearchOptions::search_for(args.name)
        .with_text(args.text)
        .include_reprints(args.reprints)
        .include_noncollectibles(args.all);
    let cards = card::lookup(&opts)?;

    for card in cards {
        println!("{card:#}");
        if args.image {
            println!("\tImage: {}", card.image);
        }
    }

    Ok(())
}
