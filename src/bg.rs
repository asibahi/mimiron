use anyhow::{anyhow, Context, Result};
use clap::Args;
use colored::Colorize;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::{Display, Write},
    iter,
};

use crate::card_details::MinionType;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    // Unique identifier
    id: usize,
    // slug: String,

    // basic info
    name: String,
    text: String,
    // collectible: u8,
    // class_id: u8,
    // multi_class_ids: Vec<Option<serde_json::Value>>,
    // card_type_id: u8,
    // card_set_id: u8,
    // rarity_id: Option<u8>,

    // Stats
    attack: Option<u8>,
    health: Option<u8>,
    mana_cost: u8,
    armor: Option<u8>,

    // Additional info
    minion_type_id: Option<u8>,
    multi_type_ids: Option<Vec<u8>>,
    battlegrounds: Option<BattlegroundsData>,
    child_ids: Option<Vec<usize>>,

    // Flavor
    image: String,
    // image_gold: String,
    // crop_image: Option<String>,
    // artist_name: Option<String>,
    // flavor_text: String,
    // parent_id: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattlegroundsData {
    hero: bool,
    quest: bool,
    reward: bool,
    companion_id: Option<usize>,
    image: String,
    // image_gold: String,
    tier: Option<u8>,
    upgrade_id: Option<usize>,
}

#[derive(Clone)]
pub enum BGCardType {
    Hero {
        armor: u8,
        buddy_id: usize,
        child_ids: Vec<usize>,
    },
    Minion {
        tier: u8,
        attack: u8,
        health: u8,
        text: String,
        minion_types: HashSet<MinionType>,
        upgrade_id: Option<usize>,
    },
    Quest {
        text: String,
    },
    Reward {
        text: String,
    },
    HeroPower {
        cost: u8,
        text: String,
    },
}
impl Display for BGCardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BGCardType::Hero {
                armor,
                buddy_id: _,
                child_ids: _,
            } => write!(f, "Hero with {armor} armor."),
            BGCardType::Minion {
                tier,
                attack,
                health,
                text,
                minion_types,
                upgrade_id: _,
            } => {
                write!(f, "Tier {tier} {attack}/{health} ")?;
                if minion_types.is_empty() {
                    write!(f, "minion")?;
                } else {
                    let types = minion_types.iter().join("/");
                    write!(f, "{types}")?;
                }
                if !f.alternate() {
                    write!(f, ": {text}")?;
                } else {
                    write!(f, ".\n\t{text}")?;
                }

                Ok(())
            }
            BGCardType::Quest { text } => write!(f, "Battlegrounds Quest: {text}"),
            BGCardType::Reward { text } => write!(f, "Battlegrounds Quest Reward: {text}"),
            BGCardType::HeroPower { text, cost } => {
                write!(f, "{cost}-cost Hero Power: {text}")
            }
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(from = "CardData")]
pub struct Card {
    pub id: usize,
    pub name: String,
    pub image: String,
    pub card_type: BGCardType,
}
impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = &self.name.bold();
        let img = &self.image;

        let card_info = &self.card_type;

        if f.alternate() {
            write!(f, "{name:25} {card_info:#}")?;
        } else {
            write!(f, "{name:25} {card_info}")?;
        }

        if f.alternate() {
            write!(f, "\n\tImage: {img}")?;
        }
        Ok(())
    }
}
impl From<CardData> for Card {
    fn from(c: CardData) -> Self {
        let card_type = if let Some(bg) = &c.battlegrounds {
            if bg.hero {
                BGCardType::Hero {
                    armor: c.armor.unwrap(),
                    buddy_id: bg.companion_id.unwrap(),
                    child_ids: c.child_ids.unwrap(),
                }
            } else if bg.quest {
                BGCardType::Quest { text: c.text }
            } else if bg.reward {
                BGCardType::Reward { text: c.text }
            } else if bg.tier.is_some() {
                BGCardType::Minion {
                    tier: bg.tier.unwrap(),
                    attack: c.attack.unwrap(),
                    health: c.health.unwrap(),
                    text: c.text,
                    minion_types: match (c.minion_type_id, c.multi_type_ids) {
                        (None, _) => HashSet::new(),
                        (Some(t), None) => HashSet::from([t.into()]),
                        (Some(t), Some(v)) => iter::once(t)
                            .chain(v)
                            .map(MinionType::from)
                            .collect::<HashSet<_>>(),
                    },
                    upgrade_id: bg.upgrade_id,
                }
            } else {
                BGCardType::HeroPower {
                    text: c.text,
                    cost: c.mana_cost,
                }
            }
        } else {
            BGCardType::Minion {
                tier: 1,
                attack: c.attack.unwrap(),
                health: c.health.unwrap(),
                text: c.text,
                minion_types: match (c.minion_type_id, c.multi_type_ids) {
                    (None, _) => HashSet::new(),
                    (Some(t), None) => HashSet::from([t.into()]),
                    (Some(t), Some(v)) => iter::once(t)
                        .chain(v)
                        .map(MinionType::from)
                        .collect::<HashSet<_>>(),
                },
                upgrade_id: None,
            }
        };

        Card {
            id: c.id,
            name: c.name,
            image: {
                if let Some(bg) = &c.battlegrounds {
                    bg.image.clone()
                } else {
                    c.image
                }
            },
            card_type,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardSearchResponse {
    cards: Vec<Card>,
    card_count: usize,
}

#[derive(Args)]
pub struct BGArgs {
    /// card name to search for
    card_name: String,

    /// let search include text inside text boxes and flavor text.
    #[arg(short, long)]
    text: bool,
}

pub fn run(args: BGArgs, access_token: &str) -> Result<String> {
    let search_term = args.card_name.to_lowercase();
    let agent = ureq::agent();

    let res = agent
        .get("https://us.api.blizzard.com/hearthstone/cards")
        .query("locale", "en_us")
        .query("gameMode", "battlegrounds")
        .query("textFilter", &search_term)
        .query("access_token", access_token)
        .call()
        .context("call to BG card search API failed")?
        .into_json::<CardSearchResponse>()
        .context("parsing BG card search json failed")?;

    if res.card_count == 0 {
        return Err(anyhow!("No Battlegrounds card found. Check your spelling."));
    }

    let mut cards = res
        .cards
        .into_iter()
        // filtering only cards that include the text in the name, instead of the body,
        // depending on the args.text variable
        .filter(|c| args.text || c.name.to_lowercase().contains(&search_term))
        // cards have copies in different sets
        .unique_by(|c| c.name.clone())
        .peekable();

    if cards.peek().is_none() {
        return Err(anyhow!(
            "No Battlegrounds card found with this name. Expand search to text boxes with -t."
        ));
    }

    let mut buffer = String::new();

    for card in cards {
        writeln!(buffer, "{card:#}")?;

        if let BGCardType::Hero {
            armor: _,
            buddy_id: _,
            child_ids,
        } = card.card_type
        {
            for id in child_ids {
                let res = agent
                    .get(&format!(
                        "https://us.api.blizzard.com/hearthstone/cards/{id}"
                    ))
                    .query("locale", "en_us")
                    .query("gameMode", "battlegrounds")
                    .query("access_token", access_token)
                    .call()
                    .context("call to card by id API failed")?
                    .into_json::<Card>()
                    .context("parsing BG card search by id json failed")?;

                writeln!(buffer, "\t{res}")?;
            }
        }
    }

    Ok(buffer)
}
