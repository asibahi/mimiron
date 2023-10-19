use anyhow::Result;
use clap::Args;
use mimiron::card;

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

    /// Print image links
    #[arg(short, long)]
    image: bool,
}

pub(crate) fn run(args: CardArgs) -> Result<()> {
    let opts = card::SearchOptions::new(args.name, args.text, args.reprints);
    let cards = card::lookup(&opts)?;

    for card in cards {
        println!("{card:#}");
        if args.image {
            println!("\tImage: {}", card.image);
        }
    }

    Ok(())
}
