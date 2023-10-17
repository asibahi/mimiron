use anyhow::Result;
use clap::Args;
use mimiron::ApiHandle;
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

    /// Override format/game mode provided by code (For Twist, Duels, Tavern Brawl, etc.)
    #[arg(short, long)]
    mode: Option<String>,

    /// Save deck image. Defaults to your downloads folder unless --output is set
    #[arg(short, long, conflicts_with("comp"))]
    image: bool,

    /// Choose deck image output.
    #[arg(short, long, requires("image"))]
    output: Option<PathBuf>,

    /// Format the deck in one column. Most compact horizontally.
    #[arg(short, long, requires("image"))]
    single: bool,

    /// Format the deck in three columns. Most compact vertically.
    #[arg(short, long, requires("image"), conflicts_with("single"))]
    wide: bool,

    /// Similar to Wide Format but with card text added.
    #[arg(
        short,
        long,
        requires("image"),
        conflicts_with("single"),
        conflicts_with("wide")
    )]
    text: bool,
}

pub(crate) fn run(args: DeckArgs, api: &ApiHandle) -> Result<()> {
    let mut deck = mimiron::deck::lookup(&args.code, api)?;

    // Add Band resolution.
    if let Some(band) = args.band {
        mimiron::deck::add_band(&mut deck, band, api)?;
    }

    // Deck format/mode override
    if let Some(format) = args.mode {
        deck.format = format;
    }

    // Deck compare and/or printing
    if let Some(code) = args.comp {
        let deck2 = mimiron::deck::lookup(&code, api)?;
        let deck_diff = deck.compare_with(&deck2);
        println!("{deck_diff}");
    } else {
        println!("{deck}");
    }

    if args.image {
        let opts = match args.single {
            true => mimiron::deck::ImageOptions::Single,
            _ if args.wide => mimiron::deck::ImageOptions::Wide,
            _ if args.text => mimiron::deck::ImageOptions::WithText,
            _ => mimiron::deck::ImageOptions::Groups,
        };

        let img = mimiron::deck::get_image(&deck, opts, api)?;

        let file_name = format!(
            "{} {} {}.png",
            deck.class,
            deck.format
                .chars()
                .filter_map(|c| c.is_alphanumeric().then(|| c.to_ascii_uppercase()))
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
