use anyhow::{anyhow, Context, Result};
use clap::Args;
use colored::Colorize;
use counter::Counter;
use itertools::Itertools;
use serde::Deserialize;
use std::collections::HashMap;
use std::{collections::BTreeMap, fmt::Display};

use crate::card::{get_cards_by_text, Card, CardArgs};
use crate::card_details::Class;
use crate::deck_image;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sideboard {
    pub sideboard_card: Card,
    pub cards_in_sideboard: Vec<Card>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    deck_code: String,
    pub format: String,
    pub class: Class,
    pub cards: Vec<Card>,
    // card_count: usize,
    pub sideboard_cards: Option<Vec<Sideboard>>,
}
impl Deck {
    fn compare_with(&self, other: &Self) -> DeckDifference {
        let counter1 = self.cards.clone().into_iter().collect::<Counter<_>>();
        let counter2 = other.cards.clone().into_iter().collect::<Counter<_>>();

        let deck1_uniques = counter1.clone() - counter2.clone();

        DeckDifference {
            shared_cards: (counter1.clone() - deck1_uniques.clone()).into_map(),
            deck1_code: self.deck_code.clone(),
            deck1_uniques: deck1_uniques.into_map(),
            deck2_code: other.deck_code.clone(),
            deck2_uniques: (counter2 - counter1).into_map(),
        }
    }
}
impl Display for Deck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = &self.deck_code;
        let class = &self.class.to_string().bold();
        let format = &self.format.to_uppercase().bold();
        writeln!(f, "{format:>10} {class} deck.")?;

        let cards = self
            .cards
            .iter()
            .collect::<Counter<_>>()
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        for (card, count) in cards {
            // crate::card_image::get_slug(&card, count).ok();
            let count = if count == 1 {
                String::new()
            } else {
                format!("{count}x")
            };
            writeln!(f, "{count:>4} {card}")?;
        }

        if let Some(sideboards) = &self.sideboard_cards {
            for sideboard in sideboards {
                let sideboard_name = &sideboard.sideboard_card.name;
                writeln!(f, "Sideboard of {sideboard_name}:")?;

                let cards = sideboard
                    .cards_in_sideboard
                    .iter()
                    .collect::<Counter<_>>()
                    .most_common_ordered();

                for (card, count) in cards {
                    let count = if count == 1 {
                        String::new()
                    } else {
                        format!("{count}x")
                    };
                    writeln!(f, "{count:>4} {card}")?;
                }
            }
        }

        write!(f, "{code}")?;
        Ok(())
    }
}

pub struct DeckDifference {
    pub shared_cards: HashMap<Card, usize>,

    deck1_code: String,
    pub deck1_uniques: HashMap<Card, usize>,

    deck2_code: String,
    pub deck2_uniques: HashMap<Card, usize>,
}
impl Display for DeckDifference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (card, count) in &self.shared_cards.iter().collect::<BTreeMap<_, _>>() {
            let count = if **count == 1 {
                String::new()
            } else {
                format!("{count}x")
            };
            writeln!(f, "{count:>4} {card}")?;
        }

        writeln!(f, "\n{}", self.deck1_code)?;
        for (card, count) in &self.deck1_uniques.iter().collect::<BTreeMap<_, _>>() {
            let count = if **count == 1 {
                String::new()
            } else {
                format!("{count}x")
            };
            writeln!(f, "{}{count:>3} {card}", "+".green())?;
        }

        writeln!(f, "\n{}", self.deck2_code)?;
        for (card, count) in &self.deck2_uniques.iter().collect::<BTreeMap<_, _>>() {
            let count = if **count == 1 {
                String::new()
            } else {
                format!("{count}x")
            };
            writeln!(f, "{}{count:>3} {card}", "-".red())?;
        }
        Ok(())
    }
}

fn deck_lookup(code: &str, access_token: &str, agent: &ureq::Agent) -> Result<Deck> {
    agent
        .get("https://us.api.blizzard.com/hearthstone/deck")
        .query("locale", "en_us")
        .query("code", code)
        .query("access_token", access_token)
        .call()
        .with_context(|| "call to deck code API failed. may be an invalid deck code.")?
        .into_json::<Deck>()
        .with_context(|| "parsing deck code json failed")
}

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
    output: Option<std::path::PathBuf>,

    /// Format the deck in one column.
    #[arg(short, long, requires("image"))]
    single: bool,

    /// Format the deck in three columns.
    #[arg(short, long, requires("image"), conflicts_with("single"))]
    wide: bool,
}

pub fn run(args: DeckArgs, access_token: &str, agent: &ureq::Agent) -> Result<()> {
    // Get the main deck
    let mut deck = deck_lookup(&args.code, access_token, agent)?;

    // Add Band resolution.
    // Function WILL need to be updated if new sideboard cards are printed.
    if let Some(band) = args.band {
        // Constants that might change should ETC be added to core.
        const ETC_NAME: &str = "E.T.C., Band Manager";
        const ETC_ID: usize = 90749;

        if !deck.cards.iter().any(|c| c.id == ETC_ID) {
            return Err(anyhow!("{ETC_NAME} does not exist in the deck."));
        }

        if deck.sideboard_cards.is_some() {
            return Err(anyhow!("Deck already has a Sideboard."));
        }

        let card_ids = deck.cards.into_iter().map(|c| c.id).join(",");

        let band_ids = band
            .into_iter()
            .map(|name| {
                get_cards_by_text(&CardArgs::for_name(name), access_token, agent)?
                    // Undocumented API Found by looking through playhearthstone.com card library
                    .map(|c| Ok(format!("{id}:{ETC_ID}", id = c.id)))
                    .next()
                    .unwrap()
            })
            .collect::<Result<Vec<String>>>()?
            .join(",");

        deck = agent
            .get("https://us.api.blizzard.com/hearthstone/deck")
            .query("locale", "en_us")
            .query("access_token", access_token)
            .query("ids", &card_ids)
            .query("sideboardCards", &band_ids)
            .call()
            .with_context(|| "call to deck API by card ids failed.")?
            .into_json::<Deck>()
            .with_context(|| "parsing deck json failed")?;
    }

    // Deck format/mode override
    if let Some(format) = args.mode {
        deck.format = format;
    }

    // Deck compare and/or printing
    if let Some(code) = args.comp {
        let deck2 = deck_lookup(&code, access_token, agent)?;
        let deck_diff = deck.compare_with(&deck2);
        println!("{deck_diff}");
    } else {
        println!("{deck}");
    }

    // Generate and save image
    if args.image {
        let shape = match (args.single, args.wide) {
            (true, _) => deck_image::Shape::Single,
            (_, true) => deck_image::Shape::Wide,
            _ => deck_image::Shape::Groups,
        };

        let img = deck_image::get(&deck, shape, agent)?;

        let name = format!(
            "{} {} {}.png",
            deck.class,
            deck.format
                .chars()
                .filter_map(|c| c.is_alphanumeric().then(|| c.to_ascii_uppercase()))
                .collect::<String>(),
            chrono::Local::now().format("%Y%m%d %H%M")
        );

        let save_file = if let Some(p) = args.output {
            p.join(name)
        } else {
            directories::UserDirs::new()
                .ok_or(anyhow!("couldn't get user directories"))?
                .download_dir()
                .ok_or(anyhow!("couldn't get downloads directory"))?
                .join(name)
        };

        img.save(save_file)?;
    }

    Ok(())
}
