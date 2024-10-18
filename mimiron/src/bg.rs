use crate::{
    card_details::{MinionType, get_metadata},
    get_access_token,
    localization::{Locale, Localize},
    CardSearchResponse, CardTextDisplay, AGENT,
};
use anyhow::Result;
use colored::Colorize;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::{self, Display},
    str::FromStr,
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
    spell_school_id: Option<u8>, // useful for Trinkets

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
#[allow(clippy::struct_excessive_bools)]
struct BGData {
    hero: bool,
    quest: bool,
    reward: bool,
    companion_id: Option<usize>,
    duos_only: bool,
    solos_only: bool, // Are _any_ minions or heroes Solos only?
    image: String,
    tier: Option<u8>,
    upgrade_id: Option<usize>,
}

/// Which BG pool this card is in.
///
/// On card data, this tells you where the card is legal.
/// As a search option, this tells you how to restrict the search. (So Solos would return both `Solos` AND `All` minions)
#[derive(Clone, Copy, Default)]
pub enum Pool {
    #[default]
    All,
    Duos,
    Solos,
}
impl FromStr for Pool {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s = s.to_ascii_lowercase();
        if s.starts_with('s') {
            Ok(Pool::Solos)
        } else if s.starts_with('d') {
            Ok(Pool::Duos)
        } else if s.starts_with('a') {
            Ok(Pool::All)
        } else {
            anyhow::bail!("Unknown Battlegrounds pool")
        }
    }
}

#[derive(Clone)]
// Remember to update `impl From<CardData> for Card` when adding a new type
// no clippy lint for dead public code
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
    HeroPower {
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
    Trinket {
        text: String,
        cost: u8,
        trinket_kind: u8,
    },
}
impl Localize for BGCardType {
    fn in_locale(&self, locale: Locale) -> impl Display {
        struct Inner<'a>(&'a BGCardType, Locale);

        impl Display for Inner<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fn inner(text: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    let text = text.to_console();

                    if f.alternate() {
                        write!(f, "\n{text}")?;
                    } else if f.sign_plus() {
                        // dumbass hack to get unformatted text fur get_associated_cards
                        write!(f, ": {text}")?;
                    }

                    Ok(())
                }

                let get_type =
                    |i: u8| get_metadata().types.iter().find(|det| det.id == i).unwrap().name(self.1);

                let battlegrounds = self.1.battlegrounds();

                match self.0 {
                    BGCardType::Hero { armor, .. } => {
                        let hero = get_type(3); // 3 for hero
                        write!(f, "{hero} [{armor}]")
                    }
                    BGCardType::Minion { tier, attack, health, text, minion_types, .. } => {
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
                    BGCardType::HeroPower { cost, text } => {
                        let heropower = get_type(10); // 10 for Hero Power.
                        write!(f, "({cost}) {heropower}")?;
                        inner(text, f)
                    }
                    BGCardType::Quest { text } => {
                        write!(f, "{battlegrounds} {}", self.1.quest())?;
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
                    BGCardType::Trinket { text, cost, trinket_kind } => {
                        let kind = get_metadata()
                            .spell_schools
                            .iter()
                            .find(|det| det.id == *trinket_kind)
                            .map_or(String::new(), |det| det.name(self.1));

                        let trinket = format!("{kind} {}", get_type(44)); // 44 for Trinket

                        write!(f, "{trinket} ({cost})")?;
                        inner(text, f)
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
    pub pool: Pool,
}
impl Localize for Card {
    fn in_locale(&self, locale: Locale) -> impl Display {
        struct Inner<'a>(&'a Card, Locale);

        impl Display for Inner<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let padding = 20_usize.saturating_sub(self.0.name.as_str().width());
                let name = self.0.name.bold();

                let card_info = self.0.card_type.in_locale(self.1);

                write!(f, "{name}{:padding$} ", "")?;

                if f.alternate() {
                    write!(f, "{card_info:#}")
                } else if f.sign_plus() {
                    write!(f, "{card_info:+}")
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
            Some(BGData { tier: Some(tier), .. }) if c.card_type_id == 42 => 
                BGCardType::Spell { tier: *tier, cost: c.mana_cost, text: c.text },
            Some(BGData { tier: Some(tier), upgrade_id, .. }) => BGCardType::Minion {
                tier: *tier,
                attack: c.attack.unwrap_or_default(),
                health: c.health.unwrap_or_default(),
                text: c.text,
                minion_types: c
                    .minion_type_id
                    .into_iter()
                    .chain(c.multi_type_ids.into_iter().flatten())
                    .filter_map(|id| MinionType::try_from(id).ok())
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
            _ if c.card_type_id == 10 => BGCardType::HeroPower { text: c.text, cost: c.mana_cost },

            _ if c.card_type_id == 44 => BGCardType::Trinket {
                text: c.text,
                cost: c.mana_cost,
                // 11 is Lesser. 12 is Greater. 9 is Tavern, 10 is Spellcraft.
                trinket_kind: c.spell_school_id.unwrap_or(11),
            },

            _ => BGCardType::Spell { tier: 0, cost: c.mana_cost, text: c.text },
        };

        let pool = c
            .battlegrounds
            .as_ref()
            .and_then(|bg| {
                bg.duos_only.then_some(Pool::Duos).or(bg.solos_only.then_some(Pool::Solos))
            })
            .unwrap_or_default();

        Self {
            id: c.id,
            name: c.name,
            image: c.battlegrounds.map_or(c.image, |bg| bg.image),
            card_type,
            pool,
        }
    }
}

pub struct SearchOptions {
    search_term: Option<String>,
    tier: Option<u8>,
    minion_type: Option<MinionType>,
    pool: Pool,
    with_text: bool,
    locale: Locale,
}

impl SearchOptions {
    #[must_use]
    pub fn empty() -> Self {
        // The reason we're not just deriving and using Default here
        // is to make it clear that it is 0 filters.
        Self {
            search_term: None,
            tier: None,
            minion_type: None,
            pool: Pool::All,
            with_text: false,
            locale: Locale::enUS,
        }
    }
    #[must_use]
    pub fn search_for(self, search_term: Option<String>) -> Self {
        Self { search_term, ..self }
    }
    #[must_use]
    pub fn with_tier(self, tier: Option<u8>) -> Self {
        Self { tier, ..self }
    }
    #[must_use]
    pub fn with_type(self, minion_type: Option<MinionType>) -> Self {
        Self { minion_type, ..self }
    }
    #[must_use]
    pub fn with_text(self, with_text: bool) -> Self {
        Self { with_text, ..self }
    }
    #[must_use]
    pub fn with_locale(self, locale: Locale) -> Self {
        Self { locale, ..self }
    }
    #[must_use]
    pub fn for_pool(self, pool: Pool) -> Self {
        Self { pool, ..self }
    }
}

pub fn lookup(opts: &SearchOptions) -> Result<impl Iterator<Item = Card> + '_> {
    let mut res = AGENT
        .get("https://us.api.blizzard.com/hearthstone/cards")
        .query("access_token", get_access_token())
        .query("locale", opts.locale.to_string())
        .query("gameMode", "battlegrounds");

    if let Some(t) = &opts.search_term {
        res = res.query("textFilter", t);
    }

    if let Some(t) = &opts.minion_type {
        res = res.query(
            "minionType",
            t.in_en_us() // Is it always enUS?
                .to_string()
                .to_lowercase()
                .replace(' ', ""),
        );
    }

    if let Some(t) = opts.tier {
        res = res.query("tier", t.to_string());
    }

    let res = res.call()?.body_mut().read_json::<CardSearchResponse<Card>>()?;

    anyhow::ensure!(res.card_count > 0, "No Battlegrounds card found. Check your spelling.");

    let mut cards = res
        .cards
        .into_iter()
        // filtering only cards that include the text in the name, instead of the body,
        // depending on the args.text variable
        .filter(|c| {
            opts.with_text
                || opts
                    .search_term
                    .as_ref()
                    .is_none_or(|name| c.name.to_lowercase().contains(&name.to_lowercase()))
        })
        .filter(|c| match opts.pool {
            Pool::All => true,
            Pool::Duos => matches!(c.pool, Pool::All | Pool::Duos),
            Pool::Solos => matches!(c.pool, Pool::All | Pool::Solos),
        })
        .sorted_by_key(|c| {
            !c.name
                .to_lowercase()
                .starts_with(&opts.search_term.as_deref().unwrap_or_default().to_lowercase())
        })
        .peekable();

    anyhow::ensure!(
        cards.peek().is_some(),
        "No Battlegrounds card found with this name. Try expanding search to text boxes."
    );

    Ok(cards)
}

#[must_use]
pub fn get_and_print_associated_cards(card: &Card, locale: Locale) -> Vec<Card> {
    let mut cards = vec![];

    match &card.card_type {
        BGCardType::Hero { buddy_id, child_ids, .. } => {
            'heropower: {
                // Getting the starting hero power only. API sometimes has outdated HPs.
                // The smallest ChildID Hero Power is (usually) the correct hero power.
                // Hope we don't get rate limited ...
                let Some(res) = child_ids
                    .iter()
                    .sorted()
                    .filter_map(|id| get_card_by_id(*id, locale).ok())
                    .find(|c| matches!(c.card_type, BGCardType::HeroPower { .. }))
                else {
                    break 'heropower;
                };

                let text = textwrap::fill(
                    &format!("{:+}", res.in_locale(locale)),
                    textwrap::Options::new(textwrap::termwidth() - 10)
                        .initial_indent("\t")
                        .subsequent_indent(&format!("\t{:<20} ", " ")),
                )
                .blue();
                println!("{text}");

                cards.push(res);
            }

            'buddy: {
                let Some(res) = buddy_id.and_then(|id| get_card_by_id(id, locale).ok()) else {
                    break 'buddy;
                };

                let text = textwrap::fill(
                    &format!("{:+}", res.in_locale(locale)),
                    textwrap::Options::new(textwrap::termwidth() - 10)
                        .initial_indent("\t")
                        .subsequent_indent(&format!("\t{:<20} ", " ")),
                )
                .green();
                println!("{text}");

                cards.push(res);
            }
        }
        BGCardType::Minion { upgrade_id: Some(id), .. } => 'golden: {
            let Ok(res) = get_card_by_id(*id, locale) else {
                break 'golden;
            };

            let BGCardType::Minion { attack, health, text, .. } = &res.card_type else {
                break 'golden;
            };

            let upgraded = format!("\t{}: {attack}/{health}", locale.golden()).italic().yellow();
            println!("{upgraded}");

            let text = text.to_console().yellow();
            println!("{text}");

            cards.push(res);
        }

        _ => (),
    }

    cards
}

fn get_card_by_id(id: usize, locale: Locale) -> Result<Card> {
    let res = AGENT
        .get(format!("https://us.api.blizzard.com/hearthstone/cards/{id}"))
        .query("locale", locale.to_string())
        .query("gameMode", "battlegrounds")
        .query("access_token", get_access_token())
        .call()?
        .body_mut()
        .read_json::<Card>()?;
    Ok(res)
}
