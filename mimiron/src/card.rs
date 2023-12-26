use crate::{
    card_details::{get_set_by_id, CardType, Class, MinionType, Rarity, RuneCost, SpellSchool},
    get_access_token, get_agent,
    helpers::prettify,
};
use anyhow::{anyhow, Result};
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CardData {
    // Unique identifiers
    id: usize,

    // basic info
    card_type_id: Option<u8>,
    class_id: Option<u8>,
    multi_class_ids: Vec<u8>,

    rarity_id: Option<u8>,
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
    flavor_text: String,
}

#[derive(Deserialize, Clone)]
#[serde(from = "CardData")]
pub struct Card {
    pub id: usize,
    pub card_set: String,

    pub name: String,
    pub class: HashSet<Class>,

    pub cost: u8,
    pub rune_cost: Option<RuneCost>,

    pub card_type: CardType,
    pub rarity: Rarity,

    pub text: String,

    pub image: String,
    pub crop_image: Option<String>,
    pub flavor_text: String,
}

impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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

        let set = &self.card_set;
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
            write!(f, " {set}\n{text}")?;
        }
        Ok(())
    }
}
impl From<CardData> for Card {
    fn from(c: CardData) -> Self {
        Self {
            id: c.id,
            card_set: get_set_by_id(c.card_set_id),
            name: c.name,
            class: if c.multi_class_ids.is_empty() {
                HashSet::from([c.class_id.unwrap_or_default().into()])
            } else {
                c.multi_class_ids
                    .into_iter()
                    .map(Class::from)
                    .collect::<HashSet<_>>()
            },
            cost: c.mana_cost,
            rune_cost: c.rune_cost,
            card_type: match c.card_type_id.unwrap_or_default() {
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
                10 => CardType::HeroPower,
                _ => CardType::Unknown,
            },
            rarity: c.rarity_id.unwrap_or_default().into(),
            text: c.text,

            image: c.image,
            crop_image: c.crop_image,
            flavor_text: c.flavor_text,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CardSearchResponse {
    cards: Vec<Card>,
    card_count: usize,
}

pub struct SearchOptions {
    search_term: String,
    with_text: bool,
    reprints: bool,
    noncollectibles: bool,
}

impl SearchOptions {
    #[must_use]
    pub fn search_for(search_term: String) -> Self {
        Self {
            search_term,
            with_text: false,
            reprints: false,
            noncollectibles: false,
        }
    }
    #[must_use]
    pub fn with_text(self, with_text: bool) -> Self {
        Self { with_text, ..self }
    }
    #[must_use]
    pub fn include_reprints(self, reprints: bool) -> Self {
        Self { reprints, ..self }
    }
    #[must_use]
    pub fn include_noncollectibles(self, noncollectibles: bool) -> Self {
        Self {
            noncollectibles,
            ..self
        }
    }
}

pub fn lookup(opts: &SearchOptions) -> Result<impl Iterator<Item = Card> + '_> {
    let search_term = &opts.search_term;

    let mut res = get_agent()
        .get("https://us.api.blizzard.com/hearthstone/cards")
        .query("locale", "en-US")
        .query("textFilter", search_term)
        .query("access_token", &get_access_token());

    if opts.noncollectibles {
        res = res.query("collectible", "0,1");
    }

    let res = res.call()?.into_json::<CardSearchResponse>()?;

    if res.card_count == 0 {
        return Err(anyhow!(
            "No constructed card found with text {search_term}. Check your spelling."
        ));
    }

    let mut cards = res
        .cards
        .into_iter()
        // filtering only cards that include the text in the name, instead of the body,
        // depending on the opts.with_text variable
        .filter(|c| opts.with_text || c.name.to_lowercase().contains(&search_term.to_lowercase()))
        // cards have copies in different sets
        .unique_by(|c| opts.reprints.either(c.id, c.name.clone()))
        .sorted_by_key(|c| {
            !c.name
                .to_lowercase()
                .starts_with(&search_term.to_lowercase())
        })
        .peekable();

    if cards.peek().is_none() {
        return Err(anyhow!(
            "No constructed card found with name \"{search_term}\". Try expanding search to text boxes."
        ));
    }

    Ok(cards)
}
