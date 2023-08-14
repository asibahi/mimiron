use anyhow::{anyhow, Context, Result};
use clap::{ArgGroup, Args};
use colored::Colorize;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::{Display, Write},
    iter,
    str::FromStr,
};

use crate::card_details::MinionType;
use crate::prettify::prettify;

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
            Self::Hero {
                armor,
                buddy_id: _,
                child_ids: _,
            } => write!(f, "Hero with {armor} armor."),
            Self::Minion {
                tier,
                attack,
                health,
                text,
                minion_types,
                upgrade_id: _,
            } => {
                let text = prettify(text);

                write!(f, "Tier-{tier} {attack}/{health} ")?;
                if minion_types.is_empty() {
                    write!(f, "minion")?;
                } else {
                    let types = minion_types.iter().join("/");
                    write!(f, "{types}")?;
                }
                if f.alternate() {
                    let text = textwrap::fill(
                        &text,
                        textwrap::Options::new(textwrap::termwidth() - 10)
                            .initial_indent("\t")
                            .subsequent_indent("\t"),
                    );
                    write!(f, ".\n{text}")?;
                } else {
                    write!(f, ": {text}")?;
                }

                Ok(())
            }
            Self::Quest { text } => {
                let text = prettify(text);
                write!(f, "Battlegrounds Quest: {text}")
            }
            Self::Reward { text } => {
                let text = prettify(text);
                write!(f, "Battlegrounds Reward: {text}")
            }
            Self::HeroPower { text, cost } => {
                let text = prettify(text);
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

        let card_info = &self.card_type;

        if f.alternate() {
            write!(f, "{name:20} {card_info:#}")?;
        } else {
            write!(f, "{name:20} {card_info}")?;
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
                    buddy_id: bg.companion_id.unwrap_or(0),
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

        Self {
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
#[command(group = ArgGroup::new("search").required(true).multiple(true))]
pub struct BGArgs {
    /// Text to search for
    #[arg(group = "search")]
    name: Option<String>,

    /// Search by Minion Battlegrounds tier
    #[arg(short, long, group = "search", value_parser = clap::value_parser!(u8).range(1..=6))]
    tier: Option<u8>,

    // Search by Minion type
    #[arg(short = 'T', long = "type", group = "search", value_parser = MinionType::from_str)]
    minion_type: Option<MinionType>,

    /// Include text inside text boxes.
    #[arg(long)]
    text: bool,

    /// Print image links.
    #[arg(short, long)]
    image: bool,
}

pub fn run(args: BGArgs, access_token: &str) -> Result<String> {
    let agent = ureq::agent();

    let mut res = agent
        .get("https://us.api.blizzard.com/hearthstone/cards")
        .query("access_token", access_token)
        .query("locale", "en_us")
        .query("gameMode", "battlegrounds");

    if let Some(t) = &args.name {
        res = res.query("textFilter", t);
    }

    if let Some(t) = args.minion_type {
        res = res.query("minionType", &t.to_string().to_lowercase());
    }

    if let Some(t) = args.tier {
        res = res.query("tier", &t.to_string());
    }

    let res = res
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
        .filter(|c| {
            args.text
                || args.name.is_none()
                || c.name.to_lowercase().contains(args.name.as_ref().unwrap())
        })
        .peekable();

    if cards.peek().is_none() {
        return Err(anyhow!(
            "No Battlegrounds card found with this name. Expand search to text boxes with --text."
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

                let res = textwrap::fill(
                    &res.to_string(),
                    textwrap::Options::new(textwrap::termwidth() - 10)
                        .initial_indent("\t")
                        .subsequent_indent(&format!("\t{:<20} ", " ")),
                );

                writeln!(buffer, "{res}")?;
            }
        }

        if args.image {
            writeln!(buffer, "\tImage: {}", card.image)?;
        }
    }

    Ok(buffer)
}
