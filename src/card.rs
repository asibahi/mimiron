use anyhow::{anyhow, Context, Result};
use clap::Args;
use colored::Colorize;
use eitherable::Eitherable;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    cmp::Ordering,
    collections::HashSet,
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
};
use unicode_width::UnicodeWidthStr;

use crate::{card_details::*, helpers::prettify, Api};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CardData {
    // Unique identifiers
    id: usize,
    // slug: String,

    // basic info

    // collectible: u8,
    card_type_id: u8,
    class_id: u8,
    multi_class_ids: Vec<u8>,

    rarity_id: u8,
    card_set_id: usize,

    name: String,
    text: String,

    // Stats
    mana_cost: u8,
    rune_cost: Option<RuneCost>,

    attack: Option<u8>,
    health: Option<u8>,
    durability: Option<u8>,
    armor: Option<u8>,

    // Additional Info
    minion_type_id: Option<u8>,
    multi_type_ids: Option<Vec<u8>>,

    spell_school_id: Option<u8>,

    // Flavor
    image: String,
    crop_image: Option<String>,
    //artist_name: String,
    //flavor_text: String,
}

#[derive(Deserialize, Clone)]
#[serde(from = "CardData")]
pub struct Card {
    pub id: usize,
    pub card_set: usize,

    pub name: String,
    pub class: HashSet<Class>,

    pub cost: u8,
    pub rune_cost: Option<RuneCost>,

    pub card_type: CardType,
    pub rarity: Rarity,

    pub text: String,

    pub image: String,

    pub crop_image: Option<String>,
}

impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cost.cmp(&other.cost).then(self.name.cmp(&other.name)))
    }
}
impl Hash for Card {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
impl Eq for Card {}
impl Ord for Card {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cost.cmp(&other.cost).then(self.name.cmp(&other.name))
    }
}
impl Display for Card {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let padding = 25_usize.saturating_sub(self.name.as_str().width());

        let name = self.name.bold();
        let cost = self.cost;

        let runes = match &self.rune_cost {
            Some(r) => format!("{r} "),
            None => String::new(),
        };

        let set = self.card_set;
        let text = prettify(&self.text);
        let text = textwrap::fill(
            &text,
            textwrap::Options::new(textwrap::termwidth() - 10)
                .initial_indent("\t")
                .subsequent_indent("\t"),
        );

        let rarity = &self.rarity;

        let class = self.class.iter().join("/");

        let card_info = &self.card_type;

        write!(
            f,
            "{name}{:padding$} {rarity} {class} {runes}{cost} mana {card_info}.",
            ""
        )?;

        if f.alternate() {
            write!(f, " Set {set}.\n{text}")?;
        }
        Ok(())
    }
}
impl From<CardData> for Card {
    fn from(c: CardData) -> Self {
        Self {
            id: c.id,
            card_set: c.card_set_id,
            name: c.name.clone(),
            class: if c.multi_class_ids.is_empty() {
                HashSet::from([c.class_id.into()])
            } else {
                c.multi_class_ids
                    .into_iter()
                    .map(Class::from)
                    .collect::<HashSet<_>>()
            },
            cost: c.mana_cost,
            rune_cost: c.rune_cost,
            card_type: match c.card_type_id {
                3 => CardType::Hero {
                    armor: c.armor.unwrap_or_default(),
                },
                4 => CardType::Minion {
                    attack: c.attack.unwrap_or_default(),
                    health: c.health.unwrap_or_default(),
                    minion_types: c
                        .minion_type_id
                        .into_iter()
                        .chain(c.multi_type_ids.into_iter().flatten())
                        .map(MinionType::from)
                        .collect(),
                },
                5 => CardType::Spell {
                    school: c.spell_school_id.map(SpellSchool::from),
                },
                7 => CardType::Weapon {
                    attack: c.attack.unwrap_or_default(),
                    durability: c.durability.unwrap_or_default(),
                },
                39 => CardType::Location {
                    durability: c.health.unwrap_or_default(),
                },
                _ => CardType::Unknown,
            },
            rarity: c.rarity_id.into(),
            text: c.text,

            image: c.image,

            crop_image: c.crop_image,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CardSearchResponse {
    cards: Vec<Card>,
    card_count: usize,
}

#[derive(Args)]
pub struct CardArgs {
    /// Text to search for
    name: String,

    /// Include text inside text boxes and flavor text
    #[arg(short, long)]
    text: bool,

    /// Include reprints
    #[arg(short, long)]
    reprints: bool,

    /// Print image links
    #[arg(short, long)]
    image: bool,
}
impl CardArgs {
    pub(crate) fn for_name(name: String) -> Self {
        Self {
            name,
            text: false,
            image: false,
            reprints: false,
        }
    }
}

pub(crate) fn run(args: CardArgs, api: &Api) -> Result<()> {
    let cards = get_cards_by_text(&args, api)?;

    for card in cards {
        println!("{card:#}");
        if args.image {
            println!("\tImage: {}", card.image);
        }
    }

    Ok(())
}

pub(crate) fn get_cards_by_text<'c>(
    args: &'c CardArgs,
    api: &Api,
) -> Result<impl Iterator<Item = Card> + 'c> {
    let search_term = &args.name;

    let res = api
        .agent
        .get("https://us.api.blizzard.com/hearthstone/cards")
        .query("locale", api.locale)
        .query("textFilter", search_term)
        .query("access_token", api.access_token)
        .call()
        .with_context(|| "call to card search API failed")?
        .into_json::<CardSearchResponse>()
        .with_context(|| "parsing card search json failed")?;

    if res.card_count == 0 {
        return Err(anyhow!(
            "No constructed card found with text {search_term}. Check your spelling."
        ));
    }

    let mut cards = res
        .cards
        .into_iter()
        // filtering only cards that include the text in the name, instead of the body,
        // depending on the args.text variable
        .filter(move |c| args.text || c.name.to_lowercase().contains(&search_term.to_lowercase()))
        // cards have copies in different sets
        .unique_by(|c| args.reprints.either(c.id, c.name.clone()))
        .peekable();

    if cards.peek().is_none() {
        return Err(anyhow!(
            "No constructed card found with name \"{search_term}\". Expand search to text boxes with --text."
        ));
    }

    Ok(cards)
}
