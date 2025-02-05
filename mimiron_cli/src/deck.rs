use anyhow::Result;
use clap::{Args, ValueEnum};
use mimiron::{
    deck::{self, LookupOptions},
    localization::{Locale, Localize},
};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

#[derive(Args, Clone)]
pub struct DeckArgs {
    /// Deck code to parse
    input: String,

    /// Compare with a second deck
    #[arg(short, long, value_name("DECK2"))]
    comp: Option<String>,

    /// Instead of a code, specify a file with multiple deck codes (separated by new lines).
    #[arg(long, conflicts_with("comp"))]
    batch: bool,

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
    /// Adapt:  Regular but adapts to deck size..
    #[arg(short, long, default_value("square"), requires("image"), verbatim_doc_comment)]
    format: ImageFormat,
}

#[derive(Clone, ValueEnum)]
enum ImageFormat { Groups, Single, Square, Wide, Adapt }

pub fn run(args: DeckArgs, locale: Locale) -> Result<()> {
    if args.batch {
        let file = BufReader::new(File::open(&args.input)?);
        for line in file.lines() {
            let line = line?;
            let args = DeckArgs { input: line.clone(), ..args.clone() };
            if let Err(e) = run_one(args, locale) {
                eprintln!("{e} in \"{line}\"");
            }
        }
    } else {
        run_one(args, locale)?;
    }

    Ok(())
}

pub fn run_one(args: DeckArgs, locale: Locale) -> Result<()> {
    let opts = LookupOptions::lookup(&args.input).with_locale(locale).with_custom_format(args.mode.as_deref());

    let deck = deck::lookup(opts)?;

    // Deck compare and/or printing
    if let Some(code) = args.comp {
        let deck2 = deck::lookup(LookupOptions::lookup(&code).with_locale(locale))?;
        let deck_diff = deck.compare_with(&deck2);
        println!("{}", deck_diff.in_locale(locale));
    } else {
        println!("{}", deck.in_locale(locale));
    }

    if args.image {
        let opts = match args.format {
            ImageFormat::Groups => deck::ImageOptions::Groups,
            ImageFormat::Adapt => deck::ImageOptions::Adaptable,
            ImageFormat::Single =>
                deck::ImageOptions::Regular { columns: 1, inline_sideboard: true },
            ImageFormat::Square =>
                deck::ImageOptions::Regular { columns: 2, inline_sideboard: false },
            ImageFormat::Wide =>
                deck::ImageOptions::Regular { columns: 3, inline_sideboard: false },
        };

        let img = deck.get_image(opts);

        let file_name = format!(
            "{} {} {}.png",
            deck.title,
            deck.deck_code.chars().filter(|c| c.is_alphanumeric()).collect::<String>(),
            chrono::Local::now().format("%Y%m%d %H%M")
        );

        let save_file = args
            .output
            .unwrap_or_else(||
                directories::UserDirs::new()
                    .expect("couldn't get user directories")
                    .download_dir()
                    .expect("couldn't get downloads directory")
                    .to_path_buf()
            )
            .join(file_name);

        img.save(save_file)?;
    }

    Ok(())
}
