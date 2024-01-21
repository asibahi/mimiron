use anyhow::Result;
use clap::{Args, ValueEnum};
use mimiron::{
    deck::{self, LookupOptions},
    localization::{Locale, Localize},
};
use pollster::FutureExt;
use std::path::PathBuf;

#[derive(Args)]
pub struct DeckArgs {
    /// Deck code to parse
    code: String,

    /// Compare with a second deck
    #[arg(short, long, value_name("DECK2"))]
    comp: Option<String>,

    /// Add Sideboard cards for E.T.C., Band Manager if the deck code lacks them. Make sure card names are exact.
    #[arg(
        short,
        long("addband"),
        value_name("BAND_MEMBER"),
        num_args(3),
        conflicts_with("comp")
    )]
    band: Option<Vec<String>>,

    /// Override format/game mode provided by code (For Twist, Tavern Brawl, etc.)
    #[arg(short, long)]
    mode: Option<String>,

    /// Save deck image. Defaults to your downloads folder unless --output is set
    #[arg(short, long, conflicts_with("comp"))]
    image: bool,

    /// Choose where to save the deck image
    #[arg(short, long, requires("image"))]
    output: Option<PathBuf>,

    /// Choose the format for the deck image.
    ///
    /// Groups: Separates Class and Neutral cards.
    /// Single: Regular style. Most compact horizontally.
    /// Square: Regular but over 2 columns. Default.
    /// Wide:   Regular but over 3 columns. Most compact vertically.
    /// Text:   Includes card text.
    #[arg(
        short,
        long,
        default_value("square"),
        requires("image"),
        verbatim_doc_comment
    )]
    format: ImageFormat,
}

#[derive(Clone, ValueEnum)]
enum ImageFormat {
    Groups,
    Single,
    Square,
    Wide,
    Adapt,
}

pub fn run(args: DeckArgs, locale: Locale) -> Result<()> {
    let opts = LookupOptions::lookup(args.code).with_locale(locale);

    let mut deck = if let Some(band) = args.band {
        // Add Band resolution.
        deck::add_band(&opts, band).block_on()?
    } else {
        deck::lookup(&opts).block_on()?
    };

    // Deck format/mode override
    if let Some(format) = args.mode {
        deck.format = format;
    }

    // Deck compare and/or printing
    if let Some(code) = args.comp {
        let deck2 = deck::lookup(&LookupOptions::lookup(code).with_locale(locale)).block_on()?;
        let deck_diff = deck.compare_with(&deck2);
        println!("{}", deck_diff.in_locale(locale));
    } else {
        println!("{}", deck.in_locale(locale));
    }

    if args.image {
        let opts = match args.format {
            ImageFormat::Groups => deck::ImageOptions::Groups,
            ImageFormat::Adapt => deck::ImageOptions::Adaptable,
            ImageFormat::Single => deck::ImageOptions::Regular { columns: 1 },
            ImageFormat::Square => deck::ImageOptions::Regular { columns: 2 },
            ImageFormat::Wide => deck::ImageOptions::Regular { columns: 3 },
        };

        let img = deck::get_image(&deck, locale, opts).block_on()?;

        let file_name = format!(
            "{} {} {}.png",
            deck.class.in_locale(locale),
            deck.format
                .chars()
                .filter(|c| c.is_alphanumeric())
                .map(|c| c.to_ascii_uppercase())
                .collect::<String>(),
            chrono::Local::now().format("%Y%m%d %H%M")
        );

        let save_file = args
            .output
            .unwrap_or_else(|| {
                directories::UserDirs::new()
                    .expect("couldn't get user directories")
                    .download_dir()
                    .expect("couldn't get downloads directory")
                    .to_path_buf()
            })
            .join(file_name);

        img.save(save_file)?;
    }

    Ok(())
}
