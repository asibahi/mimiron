use anyhow::{anyhow, Context, Result};
use clap::{ArgGroup, Args};
use colored::Colorize;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::{self, Display},
    str::FromStr,
};

use crate::card_details::MinionType;
use crate::helpers::prettify;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    // Unique identifier
    id: usize,
    // slug: String,

    // basic info
    name: String,
    text: String,
    card_type_id: u8,

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
        buddy_id: Option<usize>,
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
    Anomaly {
        text: String,
    },
    HeroPower {
        cost: u8,
        text: String,
    },
}
impl Display for BGCardType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn inner(text: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let text = prettify(text);

            if f.alternate() {
                let text = textwrap::fill(
                    &text,
                    textwrap::Options::new(textwrap::termwidth() - 10)
                        .initial_indent("\t")
                        .subsequent_indent("\t"),
                );
                write!(f, "\n{text}")
            } else {
                write!(f, ": {text}")
            }
        }

        match self {
            Self::Hero { armor, .. } => write!(f, "{armor} armor Hero."),
            Self::Minion {
                tier,
                attack,
                health,
                text,
                minion_types,
                ..
            } => {
                write!(f, "Tier-{tier} {attack}/{health} ")?;
                if minion_types.is_empty() {
                    write!(f, "minion")?;
                } else {
                    let types = minion_types.iter().join("/");
                    write!(f, "{types}")?;
                }
                inner(text, f)
            }
            Self::Quest { text } => {
                write!(f, "Battlegrounds Quest")?;
                inner(text, f)
            }
            Self::Reward { text } => {
                write!(f, "Battlegrounds Reward")?;
                inner(text, f)
            }
            Self::Anomaly { text } => {
                write!(f, "Battlegrounds Anomaly")?;
                inner(text, f)
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
        let card_type = match &c.battlegrounds {
            Some(bg) if bg.tier.is_some() => BGCardType::Minion {
                tier: bg.tier.unwrap_or_default(),
                attack: c.attack.unwrap_or_default(),
                health: c.health.unwrap_or_default(),
                text: c.text,
                minion_types: c
                    .minion_type_id
                    .into_iter()
                    .chain(c.multi_type_ids.into_iter().flatten())
                    .map(MinionType::from)
                    .collect(),
                upgrade_id: bg.upgrade_id,
            },
            Some(bg) if bg.hero => BGCardType::Hero {
                armor: c.armor.unwrap_or_default(),
                buddy_id: bg.companion_id.filter(|x| *x != 0),
                child_ids: c.child_ids.unwrap_or_default(),
            },
            Some(bg) if bg.quest => BGCardType::Quest { text: c.text },
            Some(bg) if bg.reward => BGCardType::Reward { text: c.text },
            _ if c.card_type_id == 43 => BGCardType::Anomaly { text: c.text },
            _ => BGCardType::HeroPower {
                text: c.text,
                cost: c.mana_cost,
            },
        };

        Self {
            id: c.id,
            name: c.name,
            image: match &c.battlegrounds {
                Some(bg) => bg.image.clone(),
                _ => c.image,
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
    #[arg(short, long, group = "search", value_parser = clap::value_parser!(u8).range(1..=7))]
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

pub fn run(args: BGArgs, access_token: &str, agent: &ureq::Agent) -> Result<()> {
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
        .with_context(|| "call to BG card search API failed")?
        .into_json::<CardSearchResponse>()
        .with_context(|| "parsing BG card search json failed")?;

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
                || args.name.as_ref().map_or(true, |name| {
                    c.name.to_lowercase().contains(&name.to_lowercase())
                })
        })
        .peekable();

    if cards.peek().is_none() {
        return Err(anyhow!(
            "No Battlegrounds card found with this name. Expand search to text boxes with --text."
        ));
    }

    for card in cards {
        println!("{card:#}");
        if args.image {
            println!("\tImage: {}", card.image);
        }

        match &card.card_type {
            BGCardType::Hero {
                buddy_id,
                child_ids,
                ..
            } => {
                'heropower: {
                    // Getting the starting hero power only. API keeps old
                    // versions of hero powers below that for some reason.
                    // First hero power is usually the smallest ID.
                    let Some(id) = child_ids.iter().min() else {
                        break 'heropower;
                    };
                    let Ok(res) = get_card_by_id(*id, access_token, agent) else {
                        break 'heropower;
                    };
                    let res = textwrap::fill(
                        &res.to_string(),
                        textwrap::Options::new(textwrap::termwidth() - 10)
                            .initial_indent("\t")
                            .subsequent_indent(&format!("\t{:<20} ", " ")),
                    )
                    .blue();

                    println!("{res}");
                }

                'buddy: {
                    let Some(buddy_id) = buddy_id else {
                        break 'buddy;
                    };
                    let Ok(res) = get_card_by_id(*buddy_id, access_token, agent) else {
                        break 'buddy;
                    };
                    let res = textwrap::fill(
                        &res.to_string(),
                        textwrap::Options::new(textwrap::termwidth() - 10)
                            .initial_indent("\t")
                            .subsequent_indent(&format!("\t{:<20} ", " ")),
                    )
                    .green();

                    println!("{res}");
                }
            }
            BGCardType::Minion {
                upgrade_id: Some(id),
                ..
            } => 'golden: {
                let Ok(res) = get_card_by_id(*id, access_token, agent) else {
                    break 'golden;
                };

                let BGCardType::Minion {
                    attack,
                    health,
                    text,
                    ..
                } = res.card_type
                else {
                    break 'golden;
                };

                let upgraded = format!("\tGolden: {attack}/{health}").italic().yellow();

                println!("{upgraded}");

                let res = textwrap::fill(
                    &prettify(&text),
                    textwrap::Options::new(textwrap::termwidth() - 10)
                        .initial_indent("\t")
                        .subsequent_indent("\t"),
                )
                .yellow();

                println!("{res}");
            }

            _ => (),
        }
    }

    Ok(())
}

fn get_card_by_id(
    id: usize,
    access_token: &str,
    agent: &ureq::Agent,
) -> Result<Card, anyhow::Error> {
    let res = agent
        .get(&format!(
            "https://us.api.blizzard.com/hearthstone/cards/{id}"
        ))
        .query("locale", "en_us")
        .query("gameMode", "battlegrounds")
        .query("access_token", access_token)
        .call()
        .with_context(|| "call to card by id API failed")?
        .into_json::<Card>()
        .with_context(|| "parsing BG card search by id json failed")?;
    Ok(res)
}
