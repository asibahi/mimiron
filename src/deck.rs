use anyhow::{anyhow, Context, Result};
use chrono::Local;
use clap::Args;
use colored::Colorize;
use counter::Counter;
use directories::UserDirs;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{collections::BTreeMap, fmt::Display};

use crate::card::Card;
use crate::card_details::Class;

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
        writeln!(f, "\n{format:>10} {class} deck.")?;

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

fn deck_lookup(code: &str, access_token: &str) -> Result<Deck> {
    ureq::get("https://us.api.blizzard.com/hearthstone/deck")
        .query("locale", "en_us")
        .query("code", code)
        .query("access_token", access_token)
        .call()
        .context("call to deck code API failed")?
        .into_json::<Deck>()
        .context("parsing deck code json failed")
}

#[derive(Args)]
pub struct DeckArgs {
    /// Deck code to parse
    code: String,

    /// Compare with a second deck
    #[arg(short, long, name = "DECK2")]
    comp: Option<String>,

    /// Save deck image
    #[arg(short, long, conflicts_with("DECK2"))]
    image: bool,

    /// Choose deck image output. Defaults to your Downloads folder.
    #[arg(short, long, requires("image"))]
    output: Option<PathBuf>,
}

pub fn run(args: DeckArgs, access_token: &str) -> Result<String> {
    let code = args.code;

    let deck = deck_lookup(&code, access_token)?;

    let answer = if let Some(code) = args.comp {
        let deck2 = deck_lookup(&code, access_token)?;
        let deck_diff = deck.compare_with(&deck2);
        format!("{deck_diff}")
    } else {
        format!("{deck}")
    };

    if args.image {
        let img = crate::card_image::get_deck_image(&deck, ureq::agent())?;

        let name = format!(
            "{} {} {}.png",
            deck.class,
            deck.format.to_uppercase(),
            Local::now().format("%Y%m%d %H%M")
        );

        let save_file = if let Some(p) = args.output {
            p.join(name)
        } else {
            UserDirs::new()
                .ok_or(anyhow!("couldn't get user directories"))?
                .download_dir()
                .ok_or(anyhow!("couldn't get downloads directory"))?
                .join(name)
        };

        img.save(save_file)?;
    }

    Ok(answer)
}
