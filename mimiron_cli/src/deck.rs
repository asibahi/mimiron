use anyhow::Result;
use clap::Args;
use mimiron::{deck, ApiHandle};
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

    #[command(flatten)]
    image_args: ImageArgs,
}

#[derive(Args)]
#[group(requires("image"), multiple(false))]
struct ImageArgs {
    /// Format the deck in one column. Most compact horizontally.
    #[arg(short, long)]
    single: bool,

    /// Format the deck in three columns. Most compact vertically.
    #[arg(short, long)]
    wide: bool,

    /// Similar to Wide Format but with card text added.
    #[arg(short, long)]
    text: bool,
}

pub(crate) fn run(args: DeckArgs, api: &ApiHandle) -> Result<()> {
    let mut deck = deck::lookup(&args.code, api)?;

    // Add Band resolution.
    if let Some(band) = args.band {
        deck::add_band(&mut deck, band, api)?;
    }

    // Deck format/mode override
    if let Some(format) = args.mode {
        deck.format = format;
    }

    // Deck compare and/or printing
    if let Some(code) = args.comp {
        let deck2 = deck::lookup(&code, api)?;
        let deck_diff = deck.compare_with(&deck2);
        println!("{deck_diff}");
    } else {
        println!("{deck}");
    }

    if args.image {
        let opts = if args.image_args.single {
            deck::ImageOptions::Regular {
                columns: 1,
                with_text: false,
            }
        } else if args.image_args.wide {
            deck::ImageOptions::Regular {
                columns: 3,
                with_text: false,
            }
        } else if args.image_args.text {
            deck::ImageOptions::Regular {
                columns: 3,
                with_text: true,
            }
        } else {
            deck::ImageOptions::Groups
        };

        let img = deck::get_image(&deck, opts, api)?;

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
