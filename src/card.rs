use anyhow::{anyhow, Context, Result};
use clap::Args;
use colored::Colorize;
use itertools::Itertools;
use serde::Deserialize;
use std::{collections::HashSet, fmt::Display, iter};

use crate::card_details::*;

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

    // Related cards
    //copy_of_card_id: Option<usize>,
    //parent_id: usize,
    //child_ids: Option<Vec<usize>>,

    //keyword_ids: Option<Vec<i64>>,
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
    /*
    tokens: HashSet<usize>,

    flavor_text: String,
    */
}

impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        /* match self.rarity.partial_cmp(&other.rarity) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        } */
        Some(self.cost.cmp(&other.cost).then(self.name.cmp(&other.name)))
    }
}
impl std::hash::Hash for Card {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
impl Eq for Card {}
impl Ord for Card {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        /* match self.rarity.cmp(&other.rarity) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        } */
        self.cost.cmp(&other.cost).then(self.name.cmp(&other.name))
    }
}
impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name.bold();
        let cost = self.cost;

        let runes = match &self.rune_cost {
            Some(r) => format!("{r} "),
            None => String::new(),
        };

        let set = self.card_set;
        let text = crate::prettify::prettify(&self.text);
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
            "{name:25} {rarity} {class} {runes}{cost} mana {card_info}."
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
            name: c.name,
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
                    armor: c.armor.unwrap(),
                },
                4 => CardType::Minion {
                    attack: c.attack.unwrap(),
                    health: c.health.unwrap(),
                    minion_types: match (c.minion_type_id, c.multi_type_ids) {
                        (None, _) => HashSet::new(),
                        (Some(t), None) => HashSet::from([t.into()]),
                        (Some(t), Some(v)) => iter::once(t)
                            .chain(v)
                            .map(MinionType::from)
                            .collect::<HashSet<_>>(),
                    },
                },
                5 => CardType::Spell {
                    school: c.spell_school_id.map(SpellSchool::from),
                },
                7 => CardType::Weapon {
                    attack: c.attack.unwrap(),
                    durability: c.durability.unwrap(),
                },
                39 => CardType::Location {
                    durability: c.health.unwrap(),
                },
                _ => CardType::Unknown,
            },
            rarity: c.rarity_id.into(),
            text: c.text,

            image: c.image,

            crop_image: c.crop_image,
            /*
            tokens: match c.child_ids {
                Some(v) => HashSet::from_iter(v),
                None => HashSet::new(),
            },
            flavor_text: c.flavor_text,
            */
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

    /// Print image links
    #[arg(short, long)]
    image: bool,
}

pub fn run(args: CardArgs, access_token: &str, agent: &ureq::Agent) -> Result<()> {
    let search_term = args.name.to_lowercase();

    let cards = get_cards_by_text(search_term, args.text, access_token, agent)?;

    for card in cards {
        println!("{card:#}");
        if args.image {
            println!("\tImage: {}", card.image);
        }
    }

    Ok(())
}

pub(crate) fn get_cards_by_text(
    search_term: String,
    include_body: bool,
    access_token: &str,
    agent: &ureq::Agent,
) -> Result<impl Iterator<Item = Card>> {
    let res = agent
        .get("https://us.api.blizzard.com/hearthstone/cards")
        .query("locale", "en_us")
        .query("textFilter", &search_term)
        .query("access_token", access_token)
        .call()
        .with_context(|| "call to card search API failed")?
        .into_json::<CardSearchResponse>()
        .with_context(|| "parsing card search json failed")?;

    if res.card_count == 0 {
        return Err(anyhow!(
            "No constructed card found with text {search_term}. Check your spelling."
        ));
    }

    let work_around_borrow_checker = search_term.clone();

    let mut cards = res
        .cards
        .into_iter()
        // filtering only cards that include the text in the name, instead of the body,
        // depending on the args.text variable
        .filter(move |c| include_body || c.name.eq_ignore_ascii_case(&work_around_borrow_checker))
        // cards have copies in different sets
        .unique_by(|c| c.name.clone())
        .peekable();

    if cards.peek().is_none() {
        return Err(anyhow!(
            "No constructed card found with name {search_term}. Expand search to all text boxes with -t."
        ));
    }

    Ok(cards)
}
