use colored::Colorize;
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
        let format = &self.format;
        writeln!(f, "{format} {class} deck.\n{code}")?;

        if self.sideboard_cards.is_some() {
            writeln!(f, "Main Deck:")?;
        }

        let cards = self.cards.iter().fold(BTreeMap::new(), |mut acc, c| {
            *acc.entry(c).or_insert(0) += 1;
            acc
        });

        for (card, count) in cards {
            let count = if count == 1 {
                String::new()
            } else {
                format!("{count}x")
            };
            writeln!(f, "{count:4} {card}")?;
        }

        if let Some(sideboards) = &self.sideboard_cards {
            for sideboard in sideboards {
                let sideboard_name = &sideboard.sideboard_card.name;
                writeln!(f, "Sideboard of {sideboard_name}:")?;

                let cards =
                    sideboard
                        .cards_in_sideboard
                        .iter()
                        .fold(BTreeMap::new(), |mut acc, c| {
                            *acc.entry(c).or_insert(0) += 1;
                            acc
                        });

                for (card, count) in cards {
                    let count = if count == 1 {
                        String::new()
                    } else {
                        format!("{count}x")
                    };
                    writeln!(f, "{count:4} {card}")?;
                }
            }
        }

        Ok(())
    }
}
