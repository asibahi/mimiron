use crate::{get_access_token, AGENT};
use colored::Colorize;
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
    str::FromStr,
};

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    sets: Vec<Set>,
    types: Vec<Details>,
    rarities: Vec<Details>,
    classes: Vec<Details>,
    minion_types: Vec<Details>,
    spell_schools: Vec<Details>,
}

#[allow(non_camel_case_types, dead_code)]
#[derive(Clone)]
pub enum Locale {
    deDE,
    enUS,
    esES,
    esMX,
    frFR,
    itIT,
    jaJP,
    koKR,
    plPL,
    ptBR,
    ruRU,
    thTH,
    zhCN,
    zhTW,
}
impl Display for Locale {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::deDE => "de_DE",
            Self::enUS => "en_US",
            Self::esES => "es_ES",
            Self::esMX => "es_MX",
            Self::frFR => "fr_FR",
            Self::itIT => "it_IT",
            Self::jaJP => "ja_JP",
            Self::koKR => "ko_KR",
            Self::plPL => "pl_PL",
            Self::ptBR => "pt_BR",
            Self::ruRU => "ru_RU",
            Self::thTH => "th_TH",
            Self::zhCN => "zh_CN",
            Self::zhTW => "zh_TW",
        };
        write!(f, "{s}")
    }
}

pub trait Localize {
    // no real need for trait. just to help make the signature consistent.
    fn in_locale(&self, locale: &Locale) -> impl Display;
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct LocalizedName {
    #[serde(rename = "de_DE")]
    deDE: String,
    #[serde(rename = "en_US")]
    enUS: String,
    #[serde(rename = "es_ES")]
    esES: String,
    #[serde(rename = "es_MX")]
    esMX: String,
    #[serde(rename = "fr_FR")]
    frFR: String,
    #[serde(rename = "it_IT")]
    itIT: String,
    #[serde(rename = "ja_JP")]
    jaJP: String,
    #[serde(rename = "ko_KR")]
    koKR: String,
    #[serde(rename = "pl_PL")]
    plPL: String,
    #[serde(rename = "pt_BR")]
    ptBR: String,
    #[serde(rename = "ru_RU")]
    ruRU: String,
    #[serde(rename = "th_TH")]
    thTH: String,
    #[serde(rename = "zh_CN")]
    zhCN: Option<String>,
    #[serde(rename = "zh_TW")]
    zhTW: String,
}
impl Localize for LocalizedName {
    fn in_locale(&self, locale: &Locale) -> String {
        match locale {
            Locale::deDE => self.deDE,
            Locale::enUS => self.enUS,
            Locale::esES => self.esES,
            Locale::esMX => self.esMX,
            Locale::frFR => self.frFR,
            Locale::itIT => self.itIT,
            Locale::jaJP => self.jaJP,
            Locale::koKR => self.koKR,
            Locale::plPL => self.plPL,
            Locale::ptBR => self.ptBR,
            Locale::ruRU => self.ruRU,
            Locale::thTH => self.thTH,
            Locale::zhCN => self.zhCN.unwrap_or(self.zhTW),
            Locale::zhTW => self.zhTW,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    slug: String,
    id: u8,
    name: LocalizedName,
}
impl Details {
    pub fn contains(&self, search_term: &str) -> bool {
        let ln = self.name;
        let st = search_term.to_lowercase().as_str();
        ln.deDE.to_lowercase().eq(st)
            || ln.enUS.to_lowercase().eq(st)
            || ln.esES.to_lowercase().eq(st)
            || ln.esMX.to_lowercase().eq(st)
            || ln.frFR.to_lowercase().eq(st)
            || ln.itIT.to_lowercase().eq(st)
            || ln.jaJP.eq(st)
            || ln.koKR.eq(st)
            || ln.plPL.to_lowercase().eq(st)
            || ln.ptBR.to_lowercase().eq(st)
            || ln.ruRU.to_lowercase().eq(st)
            || ln.thTH.eq(st)
            || ln.zhTW.eq(st)
            || ln.zhCN.is_some_and(|s| s.eq(st))
    }
}

static METADATA: Lazy<Metadata> = Lazy::new(|| {
    AGENT
        .get("https://us.api.blizzard.com/hearthstone/metadata/sets")
        .query("access_token", &get_access_token())
        .call()
        .and_then(|res| Ok(res.into_json::<Metadata>()?))
        .unwrap_or_default()
});

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Set {
    id: usize,
    name: LocalizedName,
    alias_set_ids: Option<Vec<usize>>,
}

pub(crate) fn get_set_by_id(id: usize, locale: &Locale) -> String {
    let set = METADATA.sets.iter().find(|s| {
        s.id == id
            || s.alias_set_ids
                .as_ref()
                .is_some_and(|aliases| aliases.contains(&id))
    });

    set.map_or_else(|| format!("Set {id}"), |s| s.name.in_locale(locale).clone())
}

#[derive(PartialEq, Eq, Hash, Clone, Deserialize)]
#[serde(from = "ClassData")]
pub enum Class {
    DeathKnight,
    DemonHunter,
    Druid,
    // Evoker,
    Hunter,
    Mage,
    // Monk,
    Paladin,
    Priest,
    Rogue,
    Shaman,
    Warlock,
    Warrior,
    Neutral,
    Unknown,
}
impl Localize for Class {
    fn in_locale(&self, locale: &Locale) -> impl Display {
        METADATA
            .classes
            .iter()
            .find(|det| *self == Self::from(det.id))
            .map(|det| det.name.in_locale(locale))
            .unwrap_or("Noncollectible".into())
    }
}
// impl Display for Class {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         let s = match self {
//             Self::DeathKnight => "Death Knight",
//             Self::DemonHunter => "Demon Hunter",
//             Self::Druid => "Druid",
//             // Self::Evoker => "Evoker",
//             Self::Hunter => "Hunter",
//             Self::Mage => "Mage",
//             // Self::Monk => "Monk",
//             Self::Paladin => "Paladin",
//             Self::Priest => "Priest",
//             Self::Rogue => "Rogue",
//             Self::Shaman => "Shaman",
//             Self::Warlock => "Warlock",
//             Self::Warrior => "Warrior",
//             Self::Neutral => "Neutral",
//             Self::Unknown => "UNKNOWN",
//         };
//         write!(f, "{s}")
//     }
// }
impl From<u8> for Class {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::DeathKnight,
            14 => Self::DemonHunter,
            2 => Self::Druid,
            // placeholder => Class::Evoker,
            3 => Self::Hunter,
            4 => Self::Mage,
            // placeholder => Class::Monk,
            5 => Self::Paladin,
            6 => Self::Priest,
            7 => Self::Rogue,
            8 => Self::Shaman,
            9 => Self::Warlock,
            10 => Self::Warrior,
            12 => Self::Neutral,
            _ => Self::Unknown,
        }
    }
}
impl From<ClassData> for Class {
    fn from(value: ClassData) -> Self {
        value.id.into()
    }
}
impl Class {
    #[must_use]
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            // colors from d0nkey.top
            Self::DeathKnight => (108, 105, 154),
            Self::DemonHunter => (37, 111, 61),
            Self::Druid => (255, 127, 14),
            Self::Hunter => (44, 160, 44),
            Self::Mage => (23, 190, 207),
            Self::Paladin => (240, 189, 39),
            Self::Priest => (200, 200, 200),
            Self::Rogue => (127, 127, 127),
            Self::Shaman => (43, 125, 180),
            Self::Warlock => (162, 112, 153),
            Self::Warrior => (200, 21, 24),
            Self::Neutral | Self::Unknown => (169, 169, 169),
        }
    }
}

#[derive(Deserialize)]
struct ClassData {
    id: u8,
}

#[derive(Clone, PartialEq)]
pub enum Rarity {
    Legendary,
    Epic,
    Rare,
    Common,
    Free,
    Noncollectible,
}
impl Localize for Rarity {
    fn in_locale(&self, locale: &Locale) -> impl Display {
        METADATA
            .rarities
            .iter()
            .find(|det| *self == Self::from(det.id))
            .map(|det| det.name.in_locale(locale))
            .unwrap_or("Noncollectible".into())
    }
}
// impl Display for Rarity {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         let r = match self {
//             Self::Common => "common".white(),
//             Self::Free => "free".white(),
//             Self::Rare => "rare".blue(),
//             Self::Epic => "epic".purple(),
//             Self::Legendary => "LEGENDARY".bright_yellow().bold(),
//             Self::Noncollectible => "Noncollectible".clear(),
//         }
//         .italic();
//         write!(f, "{r}")
//     }
// }
impl From<u8> for Rarity {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Common,
            2 => Self::Free,
            3 => Self::Rare,
            4 => Self::Epic,
            5 => Self::Legendary,
            _ => Self::Noncollectible,
        }
    }
}
impl Rarity {
    #[must_use]
    pub fn color(&self) -> (u8, u8, u8) {
        // colors from https://wowpedia.fandom.com/wiki/Quality
        match self {
            Self::Legendary => (255, 128, 0),
            Self::Epic => (163, 53, 238),
            Self::Rare => (0, 112, 221),
            Self::Common | Self::Free => (157, 157, 157),
            Self::Noncollectible => (0, 204, 255),
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum SpellSchool {
    Arcane,
    Fire,
    Frost,
    Nature,
    Holy,
    Shadow,
    Fel,
}
impl Localize for SpellSchool {
    fn in_locale(&self, locale: &Locale) -> impl Display {
        METADATA
            .spell_schools
            .iter()
            .find(|det| *self == Self::from(det.id))
            .map(|det| det.name.in_locale(locale))
            .unwrap_or("UNKNOWN".into())
    }
}
// impl Display for SpellSchool {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         let s = match self {
//             Self::Arcane => "Arcane",
//             Self::Fire => "Fire",
//             Self::Frost => "Frost",
//             Self::Nature => "Nature",
//             Self::Holy => "Holy",
//             Self::Shadow => "Shadow",
//             Self::Fel => "Fel",
//             Self::Unknown => "UNKNOWN",
//         };

//         write!(f, "{s}")
//     }
// }
impl From<u8> for SpellSchool {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Arcane,
            2 => Self::Fire,
            3 => Self::Frost,
            4 => Self::Nature,
            5 => Self::Holy,
            6 => Self::Shadow,
            _ => Self::Fel, // 7
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum MinionType {
    Undead,
    Murloc,
    Demon,
    Mech,
    Elemental,
    Beast,
    Totem,
    Pirate,
    Dragon,
    All,
    Quilboar,
    Naga,
}
impl Localize for MinionType {
    fn in_locale(&self, locale: &Locale) -> impl Display {
        METADATA
            .minion_types
            .iter()
            .find(|det| *self == Self::from(det.id))
            .map(|det| det.name.in_locale(locale))
            .unwrap_or("UNKNOWN".into())
    }
}
// impl Display for MinionType {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         let t = match self {
//             Self::Undead => "Undead",
//             Self::Murloc => "Murloc",
//             Self::Demon => "Demon",
//             Self::Mech => "Mech",
//             Self::Elemental => "Elemental",
//             Self::Beast => "Beast",
//             Self::Totem => "Totem",
//             Self::Pirate => "Pirate",
//             Self::Dragon => "Dragon",
//             Self::All => "Amalgam",
//             Self::Quilboar => "Quilboar",
//             Self::Naga => "Naga",
//             Self::Unknown => "UNKNOWN",
//         };
//         write!(f, "{t}")
//     }
// }
impl From<u8> for MinionType {
    fn from(value: u8) -> Self {
        match value {
            11 => Self::Undead,
            14 => Self::Murloc,
            15 => Self::Demon,
            17 => Self::Mech,
            18 => Self::Elemental,
            20 => Self::Beast,
            21 => Self::Totem,
            23 => Self::Pirate,
            24 => Self::Dragon,
            26 => Self::All,
            43 => Self::Quilboar,
            _ => Self::Naga, // 92 but who cares
        }
    }
}
impl FromStr for MinionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase().as_str();

        METADATA
            .minion_types
            .iter()
            .find(|det| det.contains(s))
            .map(|det| MinionType::from(det.id))
            .ok_or_else(|| anyhow::anyhow!("Not a valid minion type (yet?)"))
    }
}

#[derive(Deserialize, Clone)]
pub struct RuneCost {
    blood: u8,
    frost: u8,
    unholy: u8,
}
impl Display for RuneCost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (0..self.blood)
            .map(|_| "B".red())
            .chain((0..self.frost).map(|_| "F".blue()))
            .chain((0..self.unholy).map(|_| "U".green()))
            .try_for_each(|c| write!(f, "{c}"))
    }
}

#[derive(Clone)]
pub enum CardType {
    Hero {
        armor: u8,
    },
    Minion {
        attack: u8,
        health: u8,
        minion_types: HashSet<MinionType>,
    },
    Spell {
        school: Option<SpellSchool>,
    },
    Weapon {
        attack: u8,
        durability: u8,
    },
    Location {
        durability: u8,
    },
    HeroPower,
    Unknown,
}
impl Localize for CardType {
    fn in_locale(&self, locale: &Locale) -> impl Display {
        // Wizardry. Implement an InnerType that implements Display with all its
        // ergonomics, and return it!!
        struct Inner<'a, 'b>(&'a CardType, &'b Locale);

        impl Display for Inner<'_, '_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                // f.alternate() is used for text boxes on images. Regular mode for console output.
                let colon = if f.alternate() { ":" } else { "" };

                let get_type = |i: u8| {
                    // all this just to say "Minion"
                    METADATA
                        .types
                        .iter()
                        .find(|det| det.id == i)
                        .unwrap()
                        .name
                        .in_locale(self.1)
                };

                match self.0 {
                    CardType::Hero { armor } => {
                        let hero = get_type(3); // 3 for Hero
                        write!(f, "{hero} [{armor}]{colon}")
                    }
                    CardType::Minion {
                        attack,
                        health,
                        minion_types,
                    } => {
                        let types = minion_types.iter().map(|t| t.in_locale(self.1)).join("/");
                        let blurp = if types.is_empty() { get_type(4) } else { types }; // 4 for Minion
                        write!(f, "{attack}/{health} {blurp}{colon}")
                    }
                    CardType::Spell { school } => {
                        let spell = get_type(5); // 5 for Spell
                        match school {
                            Some(s) => write!(f, "{} {spell}{colon}", s.in_locale(self.1)),
                            None => write!(f, "{spell}{colon}"),
                        }
                    }
                    CardType::Weapon { attack, durability } => {
                        let weapon = get_type(7); // 7 for Weapon
                        write!(f, "{attack}/{durability} {weapon}{colon}")
                    }
                    CardType::Location { durability } => {
                        let location = get_type(39); // 39 for Location
                        write!(f, "{location} /{durability}{colon}")
                    }
                    CardType::HeroPower => {
                        let heropower = get_type(10); // 10 for Hero Power
                        write!(f, "{heropower}{colon}")
                    }
                    CardType::Unknown => write!(f, "UNKNOWN{colon}"),
                }
            }
        }

        Inner(self, locale)
    }
}

// Hearthstone Json unofficial (from HearthSim)
// Uses https://hearthstonejson.com data for back up if needed.

static HEARTH_SIM_IDS: Lazy<Vec<HearthSimData>> = Lazy::new(|| {
    AGENT
        .get("https://api.hearthstonejson.com/v1/191554/enUS/cards.json")
        .call()
        .and_then(|res| Ok(res.into_json::<Vec<HearthSimData>>()?))
        .unwrap_or_default()
});

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HearthSimData {
    dbf_id: usize,
    id: String,
    name: String,
}

pub(crate) fn get_hearth_sim_id(card: &crate::card::Card) -> Option<String> {
    HEARTH_SIM_IDS
        .iter()
        .find(|c| c.dbf_id == card.id || c.name == card.name)
        .map(|c| c.id.clone())
}
