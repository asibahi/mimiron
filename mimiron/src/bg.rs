use crate::{
    card_details::{Locale, Localize, MinionType},
    get_access_token,
    helpers::prettify,
    AGENT,
};
use anyhow::{anyhow, Result};
use colored::Colorize;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::{self, Display},
};
use unicode_width::UnicodeWidthStr;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CardData {
    id: usize,

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
    battlegrounds: Option<BGData>,
    child_ids: Option<Vec<usize>>,

    // Flavor
    image: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BGData {
    hero: bool,
    quest: bool,
    reward: bool,
    companion_id: Option<usize>,
    image: String,
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
    Spell {
        tier: u8,
        cost: u8,
        text: String,
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
impl Localize for BGCardType {
    fn in_locale(&self, locale: Locale) -> impl Display {
        struct Inner<'a>(&'a BGCardType, Locale);

        impl Display for Inner<'_> {
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

                let get_type = |i: u8| {
                    // all this just to say "Minion"
                    crate::card_details::METADATA
                        .types
                        .iter()
                        .find(|det| det.id == i)
                        .unwrap()
                        .name
                        .in_locale(self.1)
                };

                let (battlegrounds, quest) = match self.1 {
                    // only use for these strings in the code base (so far?)
                    Locale::deDE => ("Schlachtfeld", "Quest"),
                    Locale::enUS => ("Battlegrounds", "Quest"),
                    Locale::esES | Locale::esMX => ("Battlegrounds", "Misión"),
                    Locale::frFR => ("Champs de bataille", "Quête"),
                    Locale::itIT => ("Battaglia", "Missione"),
                    Locale::jaJP => ("バトルグラウンド", "クエスト"),
                    Locale::koKR => ("전장", "퀘스트"),
                    Locale::plPL => ("Ustawka", "Zadanie"),
                    Locale::ptBR => ("Campos de Batalha", "Missão"),
                    Locale::ruRU => ("Поля сражений", "Задача"),
                    Locale::thTH => ("Battlegrounds", "เควสต์"),
                    Locale::zhCN => ("Battlegrounds", "任务"),
                    Locale::zhTW => ("Battlegrounds", "任務"),
                };

                match self.0 {
                    BGCardType::Hero { armor, .. } => {
                        let hero = get_type(3); // 3 for hero
                        write!(f, "{hero} [{armor}]")
                    }
                    BGCardType::Minion {
                        tier,
                        attack,
                        health,
                        text,
                        minion_types,
                        ..
                    } => {
                        let types = minion_types.iter().map(|t| t.in_locale(self.1)).join("/");
                        let blurp = if types.is_empty() { get_type(4) } else { types }; // 4 for Minion
                        write!(f, "T-{tier} {attack}/{health} {blurp}")?;
                        inner(text, f)
                    }
                    BGCardType::Spell { tier, cost, text } => {
                        let spell = get_type(5); // 5 for Spell
                        write!(f, "T-{tier}, ({cost}) {spell}")?;
                        inner(text, f)
                    }
                    BGCardType::Quest { text } => {
                        write!(f, "{battlegrounds} {quest}")?;
                        inner(text, f)
                    }
                    BGCardType::Reward { text } => {
                        let reward = get_type(40); // 40 for BGReward
                        write!(f, "{battlegrounds} {reward}")?;
                        inner(text, f)
                    }
                    BGCardType::Anomaly { text } => {
                        write!(f, "{battlegrounds} Anomaly")?; // couldnt find localization
                        inner(text, f)
                    }
                    BGCardType::HeroPower { cost, text } => {
                        let heropower = get_type(10); // 10 for Hero Power.
                        let text = prettify(text);
                        write!(f, "({cost}) {heropower}: {text}")
                    }
                }
            }
        }

        Inner(self, locale)
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
impl Localize for Card {
    fn in_locale(&self, locale: Locale) -> impl Display {
        struct Inner<'a>(&'a Card, Locale);

        impl Display for Inner<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let padding = 20_usize.saturating_sub(self.0.name.as_str().width());
                let name = &self.0.name.bold();

                let card_info = &self.0.card_type.in_locale(self.1);

                write!(f, "{name}{:padding$} ", "")?;

                if f.alternate() {
                    write!(f, "{card_info:#}")
                } else {
                    write!(f, "{card_info}")
                }
            }
        }

        Inner(self, locale)
    }
}
impl From<CardData> for Card {
    fn from(c: CardData) -> Self {
        let card_type = match &c.battlegrounds {
            Some(BGData {
                tier: Some(tier), ..
            }) if c.card_type_id == 42 => BGCardType::Spell {
                tier: *tier,
                cost: c.mana_cost,
                text: c.text,
            },
            Some(BGData {
                tier: Some(tier),
                upgrade_id,
                ..
            }) => BGCardType::Minion {
                tier: *tier,
                attack: c.attack.unwrap_or_default(),
                health: c.health.unwrap_or_default(),
                text: c.text,
                minion_types: c
                    .minion_type_id
                    .into_iter()
                    .chain(c.multi_type_ids.into_iter().flatten())
                    .map(MinionType::from)
                    .collect(),
                upgrade_id: *upgrade_id,
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
            image: c.battlegrounds.map_or(c.image, |bg| bg.image),
            card_type,
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
    search_term: Option<String>,
    tier: Option<u8>,
    minion_type: Option<MinionType>,
    with_text: bool,
    locale: Locale,
}

impl SearchOptions {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            search_term: None,
            tier: None,
            minion_type: None,
            with_text: false,
            locale: Locale::enUS,
        }
    }
    #[must_use]
    pub fn search_for(self, search_term: Option<String>) -> Self {
        Self {
            search_term,
            ..self
        }
    }
    #[must_use]
    pub fn with_tier(self, tier: Option<u8>) -> Self {
        Self { tier, ..self }
    }
    #[must_use]
    pub fn with_type(self, minion_type: Option<MinionType>) -> Self {
        Self {
            minion_type,
            ..self
        }
    }
    #[must_use]
    pub fn with_text(self, with_text: bool) -> Self {
        Self { with_text, ..self }
    }
    #[must_use]
    pub fn with_locale(self, locale: Locale) -> Self {
        Self { locale, ..self }
    }
}

pub fn lookup(opts: &SearchOptions) -> Result<impl Iterator<Item = Card> + '_> {
    let mut res = AGENT
        .get("https://us.api.blizzard.com/hearthstone/cards")
        .query("access_token", &get_access_token())
        .query("locale", &opts.locale.to_string())
        .query("gameMode", "battlegrounds");

    if let Some(t) = &opts.search_term {
        res = res.query("textFilter", t);
    }

    if let Some(t) = &opts.minion_type {
        res = res.query(
            "minionType",
            &t.in_locale(Locale::enUS).to_string(), // Is it always enUS?
        );
    }

    if let Some(t) = opts.tier {
        res = res.query("tier", &t.to_string());
    }

    let res = res.call()?.into_json::<CardSearchResponse>()?;

    if res.card_count == 0 {
        return Err(anyhow!("No Battlegrounds card found. Check your spelling."));
    }

    let mut cards = res
        .cards
        .into_iter()
        // filtering only cards that include the text in the name, instead of the body,
        // depending on the args.text variable
        .filter(|c| {
            opts.with_text
                || opts.search_term.as_ref().map_or(true, |name| {
                    c.name.to_lowercase().contains(&name.to_lowercase())
                })
        })
        .sorted_by_key(|c| {
            !c.name.to_lowercase().starts_with(
                &opts
                    .search_term
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase(),
            )
        })
        .peekable();

    if cards.peek().is_none() {
        return Err(anyhow!(
            "No Battlegrounds card found with this name. Try expanding search to text boxes."
        ));
    }

    Ok(cards)
}

#[must_use]
pub fn get_and_print_associated_cards(card: &Card, locale: Locale) -> Vec<Card> {
    let mut cards = vec![];

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
                let Ok(res) = get_card_by_id(*id, locale) else {
                    break 'heropower;
                };
                let text = textwrap::fill(
                    &res.in_locale(locale).to_string(),
                    textwrap::Options::new(textwrap::termwidth() - 10)
                        .initial_indent("\t")
                        .subsequent_indent(&format!("\t{:<20} ", " ")),
                )
                .blue();

                cards.push(res);

                println!("{text}");
            }

            'buddy: {
                let Some(buddy_id) = buddy_id else {
                    break 'buddy;
                };
                let Ok(res) = get_card_by_id(*buddy_id, locale) else {
                    break 'buddy;
                };

                let text = textwrap::fill(
                    &res.in_locale(locale).to_string(),
                    textwrap::Options::new(textwrap::termwidth() - 10)
                        .initial_indent("\t")
                        .subsequent_indent(&format!("\t{:<20} ", " ")),
                )
                .green();

                cards.push(res);

                println!("{text}");
            }
        }
        BGCardType::Minion {
            upgrade_id: Some(id),
            ..
        } => 'golden: {
            let Ok(res) = get_card_by_id(*id, locale) else {
                break 'golden;
            };

            let BGCardType::Minion {
                attack,
                health,
                text,
                ..
            } = &res.card_type
            else {
                break 'golden;
            };

            let upgraded = format!("\tGolden: {attack}/{health}").italic().yellow();

            println!("{upgraded}");

            let text = textwrap::fill(
                &prettify(text),
                textwrap::Options::new(textwrap::termwidth() - 10)
                    .initial_indent("\t")
                    .subsequent_indent("\t"),
            )
            .yellow();

            cards.push(res);

            println!("{text}");
        }

        _ => (),
    }

    cards
}

fn get_card_by_id(id: usize, locale: Locale) -> Result<Card> {
    let res = AGENT
        .get(&format!(
            "https://us.api.blizzard.com/hearthstone/cards/{id}"
        ))
        .query("locale", &locale.to_string())
        .query("gameMode", "battlegrounds")
        .query("access_token", &get_access_token())
        .call()?
        .into_json::<Card>()?;
    Ok(res)
}
