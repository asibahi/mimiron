use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use counter::Counter;
use serde::Deserialize;
use std::{collections::BTreeMap, fmt::Display};

use crate::card::Card;
use crate::card_details::Class;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sideboard {
    sideboard_card: Card,
    cards_in_sideboard: Vec<Card>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    deck_code: String,
    format: String,
    class: Class,
    cards: Vec<Card>,
    // card_count: usize,
    sideboard_cards: Option<Vec<Sideboard>>,
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
fn compare_decks(deck: Deck, deck2: Deck) {
    let counter1 = deck.cards.iter().collect::<Counter<_>>();
    let counter2 = deck2.cards.iter().collect::<Counter<_>>();

    let fst_diff = counter1.clone() - counter2.clone();
    let common_cards = (counter1.clone() - fst_diff.clone())
        .into_iter()
        .collect::<BTreeMap<_, _>>();

    let fst_diff = fst_diff.into_iter().collect::<BTreeMap<_, _>>();
    let snd_diff = (counter2 - counter1)
        .into_iter()
        .collect::<BTreeMap<_, _>>();

    for (card, count) in common_cards {
        let count = if count == 1 {
            String::new()
        } else {
            format!("{count}x")
        };
        println!("{count:>4} {card}");
    }
    println!();
    for (card, count) in fst_diff {
        let count = if count == 1 {
            String::new()
        } else {
            format!("{count}x")
        };
        println!("+{count:>3} {card}");
    }
    println!();
    for (card, count) in snd_diff {
        let count = if count == 1 {
            String::new()
        } else {
            format!("{count}x")
        };
        println!("-{count:>3} {card}");
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
}

pub fn run(args: DeckArgs, access_token: &str) -> Result<()> {
    let code = args.code;

    let deck = deck_lookup(&code, access_token)?;

    if let Some(code) = args.comp {
        let deck2 = deck_lookup(&code, access_token)?;
        compare_decks(deck, deck2);
    } else {
        println!("{deck}");
    }

    Ok(())
}
