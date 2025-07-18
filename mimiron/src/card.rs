use crate::{
    AGENT, CardSearchResponse, CardTextDisplay,
    card_details::{CardType, Class, Faction, MinionType, Rarity, RuneCost, SpellSchool},
    get_access_token,
    hearth_sim::{fuzzy_search_hearth_sim, get_hearth_sim_details},
    localization::{Locale, Localize},
};
use anyhow::Result;
use colored::Colorize;
use compact_str::{CompactString, ToCompactString, format_compact};
use eitherable::Eitherable;
use enumset::EnumSet;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
    ops::Not,
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

    name: CompactString,
    text: CompactString,

    // Stats
    mana_cost: u8,
    rune_cost: Option<RuneCost>,

    attack: Option<u8>,
    health: Option<u8>,
    durability: Option<u8>, // apparently merged with health ?
    armor: Option<u8>,

    // Additional Info
    minion_type_id: Option<u8>,
    multi_type_ids: Option<Vec<u8>>,

    spell_school_id: Option<u8>,

    faction_id: Option<Vec<usize>>,

    // Whether card is functional or cosmetic. For Zilliax Deluxe 3000.
    is_zilliax_cosmetic_module: bool,

    // Flavor
    image: CompactString,
    crop_image: Option<CompactString>,
    flavor_text: CompactString,
}

#[derive(Deserialize, Clone)]
#[serde(from = "CardData")]
pub struct Card {
    pub id: usize,
    set: usize,

    pub name: CompactString,
    pub class: EnumSet<Class>,

    pub cost: u8,
    pub rune_cost: Option<RuneCost>,

    pub card_type: CardType,
    pub rarity: Rarity,

    pub faction: Option<Faction>,

    pub text: CompactString,

    pub image: CompactString,
    pub crop_image: Option<CompactString>,
    pub flavor_text: CompactString,

    pub cosmetic: bool,
}
impl Card {
    pub(crate) fn dummy(id: usize) -> Self {
        let (name, cost, rarity) = get_hearth_sim_details(id).unwrap_or_else(
            || (format_compact!("Unknown Card ID {id}"), 99, Rarity::Noncollectible)
        );

        Self {
            id,
            set: 1635,
            name,
            class: EnumSet::empty(),
            cost,
            rune_cost: None,
            card_type: CardType::Unknown,
            rarity,
            faction: None,
            text: CompactString::default(),
            image: "https://art.hearthstonejson.com/v1/orig/GAME_006.png".into(),
            crop_image: None,
            flavor_text: CompactString::default(),
            cosmetic: false,
        }
    }
    #[must_use]
    pub fn card_set(&self, locale: Locale) -> CompactString {
        crate::card_details::get_set_by_id(self.set, locale)
    }

    pub(crate) const fn stats(&self) -> (Option<u8>, Option<u8>) {
        let (attack, health) = match self.card_type {
            CardType::Minion { attack, health, .. }
            | CardType::Weapon { attack, durability: health } => (Some(attack), Some(health)),
            CardType::Hero { armor: health } | CardType::Location { durability: health } =>
                (None, Some(health)),
            CardType::Spell { .. } | CardType::HeroPower | CardType::Unknown => (None, None),
        };

        (attack, health)
    }

    pub(crate) fn text_elements(&self) -> (CompactString, CompactString) {
        (self.name.clone(), self.text.clone())
    }
}

impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.text == other.text
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
        self.text.hash(state);
    }
}
impl Eq for Card {}
impl Ord for Card {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cost.cmp(&other.cost).then(self.name.cmp(&other.name))
    }
}
impl Localize for Card {
    fn in_locale(&self, locale: Locale) -> impl Display {
        struct Inner<'a>(&'a Card, Locale);

        impl Display for Inner<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                let padding = 25_usize.saturating_sub(self.0.name.as_str().width());

                let name = self.0.name.bold();
                let cost = self.0.cost;

                let runes = self.0.rune_cost.as_ref().map_or_else(
                    CompactString::default,
                    |r| format_compact!("{r} ")
                );

                let rarity = self.0.rarity.in_locale(self.1);
                let class = self.0.class.in_locale(self.1);

                let faction = self
                    .0
                    .faction
                    .map(|f| f.in_locale(self.1).to_compact_string())
                    .map_or_else(CompactString::default, |r| format_compact!("{r} "));

                let card_info = self.0.card_type.in_locale(self.1);

                write!(f, "{name}{:padding$} {rarity} {class} {runes}({cost}) {faction}{card_info}.", "")?;

                if f.alternate() {
                    let set = self.0.card_set(self.1);
                    let text = self.0.text.to_console();
                    write!(f, " {set}\n{text}")?;
                }
                Ok(())
            }
        }

        Inner(self, locale)
    }
}
impl From<CardData> for Card {
    fn from(c: CardData) -> Self {
        Self {
            id: c.id,
            set: c.card_set_id,
            name: c.name,
            class: c.multi_class_ids
                .into_iter()
                .chain(c.class_id)
                .filter_map(|c| Class::try_from(c).ok())
                .collect(),
            cost: c.mana_cost,
            rune_cost: c.rune_cost,
            card_type: match c.card_type_id.unwrap_or_default() {
                3 => CardType::Hero { armor: c.armor.unwrap_or_default() },
                4 => CardType::Minion {
                    attack: c.attack.unwrap_or_default(),
                    health: c.health.unwrap_or_default(),
                    minion_types: c.minion_type_id
                        .into_iter()
                        .chain(c.multi_type_ids.into_iter().flatten())
                        .filter_map(|id| MinionType::try_from(id).ok())
                        .collect(),
                },
                5 | 42 => CardType::Spell { school: c.spell_school_id.map(SpellSchool::from) },
                7 => CardType::Weapon {
                    attack: c.attack.unwrap_or_default(),
                    durability: c.durability.or(c.health).unwrap_or_default(),
                },
                39 => CardType::Location { durability: c.health.unwrap_or_default() },
                10 => CardType::HeroPower,
                _ => CardType::Unknown,
            },
            rarity: c.rarity_id.unwrap_or_default().into(),

            faction: c.faction_id.into_iter().flatten().next().map(Faction),

            text: c.text,

            image: c.image,
            crop_image: c.crop_image,
            flavor_text: c.flavor_text,

            cosmetic: c.is_zilliax_cosmetic_module,
        }
    }
}

pub struct SearchOptions<'s> {
    search_term: &'s str,
    with_text: bool,
    reprints: bool,
    noncollectibles: bool,
    locale: Locale,

    debug: bool, // for debugging
}

impl<'s> SearchOptions<'s> {
    #[must_use]
    pub const fn search_for(search_term: &'s str) -> Self {
        Self {
            search_term,
            with_text: false,
            reprints: false,
            noncollectibles: false,
            locale: Locale::enUS,

            debug: false,
        }
    }
    #[must_use]
    pub const fn with_text(self, with_text: bool) -> Self {
        Self { with_text, ..self }
    }
    #[must_use]
    pub const fn include_reprints(self, reprints: bool) -> Self {
        Self { reprints, ..self }
    }
    #[must_use]
    pub const fn include_noncollectibles(self, noncollectibles: bool) -> Self {
        Self { noncollectibles, ..self }
    }
    #[must_use]
    pub const fn with_locale(self, locale: Locale) -> Self {
        Self { locale, ..self }
    }
    #[must_use]
    pub const fn debug(self, json: bool) -> Self {
        Self { debug: json, ..self }
    }
}

pub fn lookup(opts: SearchOptions<'_>) -> Result<impl Iterator<Item = Card> + '_> {
    let search_term = opts.search_term;

    let get_res = |st| {
        let mut res = AGENT
            .get("https://us.api.blizzard.com/hearthstone/cards")
            .header("Authorization", format!("Bearer {}", get_access_token()))
            .query("locale", opts.locale.to_compact_string())
            .query("textFilter", st)
            .query("pageSize", "500");

        if opts.noncollectibles {
            res = res.query("collectible", "0,1");
        }

        res
    };

    let res = get_res(search_term);

    if opts.debug {
        let res = res.call()?.into_body().read_to_string()?;
        eprintln!("{res}");

        return Ok(vec![].into_iter().peekable())
    }

    let mut res = res.call()?.body_mut().read_json::<CardSearchResponse<Card>>()?;

    let fuzzed = if res.card_count == 0 {
        let fuzzed = fuzzy_search_hearth_sim(search_term);
        match &fuzzed {
            Some(fuzzed) if fuzzed.1 >= 150 => { // arbitrary
                res = get_res(&fuzzed.0)
                    .call()?
                    .body_mut()
                    .read_json::<CardSearchResponse<Card>>()?;
            },
            _ => {}
        }
        fuzzed
    } else { None };

    anyhow::ensure!(
        res.card_count > 0,
        "No constructed card found with name or text {search_term}. {}",
        fuzzed.map_or("Check your spelling".to_owned(), |s| format!("Did you mean \"{}\"?", s.0))
    );

    let mut cards = res
        .cards
        .into_iter()
        .filter(|c|
            // Filtering out hero portraits if not searching for incollectibles
            (opts.noncollectibles || c.set != 17)
            // Depending on opts.with_text, whether to restrict searches to card names
            // or expand to search boxes. Also if found a result with fuzzing, list it.
                && (opts.with_text
                    || fuzzed.as_ref()
                        .is_some_and(|f| c.name.to_lowercase().contains(&*f.0.to_lowercase()))
                    || c.name.to_lowercase().contains(&search_term.to_lowercase())))
        // Cards may have copies in different sets, or cards with the same name but different text (Khadgar!!)
        .unique_by(|c| opts.reprints.either(c.id, c.text_elements()))
        // when searching for Ragnaros guarantee that Ragnaros is the first result.
        .sorted_by_key(|c| c.name.to_lowercase().starts_with(&search_term.to_lowercase()).not())
        .peekable();

    anyhow::ensure!(
        cards.peek().is_some(),
        "No constructed card found with name \"{search_term}\". Try expanding search to text boxes."
    );

    Ok(cards)
}
