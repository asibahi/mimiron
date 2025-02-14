use crate::{
    get_access_token,
    keyword::Keyword,
    localization::{Locale, Localize},
    AGENT,
};
use colored::Colorize;
use compact_str::{format_compact, CompactString, ToCompactString};
use either::Either::{self, Left, Right};
use enumset::{EnumSet, EnumSetType};
use itertools::Itertools;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use serde::Deserialize;
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
    time::{Duration, Instant},
};

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Metadata {
    pub sets: Vec<Set>,
    pub types: Vec<Details<u8>>,
    pub rarities: Vec<Details<u8>>,
    pub classes: Vec<Details<u8>>,
    pub minion_types: Vec<Details<u8>>,
    pub spell_schools: Vec<Details<u8>>,
    pub factions: Vec<Details<usize>>,
    pub keywords: Vec<Keyword>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Clone)]
pub(crate) struct LocalizedName {
    #[serde(rename = "de_DE")] deDE: CompactString,
    #[serde(rename = "en_US")] enUS: CompactString,
    #[serde(rename = "es_ES")] esES: CompactString,
    #[serde(rename = "es_MX")] esMX: CompactString,
    #[serde(rename = "fr_FR")] frFR: CompactString,
    #[serde(rename = "it_IT")] itIT: CompactString,
    #[serde(rename = "ja_JP")] jaJP: CompactString,
    #[serde(rename = "ko_KR")] koKR: CompactString,
    #[serde(rename = "pl_PL")] plPL: CompactString,
    #[serde(rename = "pt_BR")] ptBR: CompactString,
    #[serde(rename = "ru_RU")] ruRU: CompactString,
    #[serde(rename = "th_TH")] thTH: CompactString,
    #[serde(rename = "zh_CN")] zhCN: Option<CompactString>,
    #[serde(rename = "zh_TW")] zhTW: CompactString,
}
impl LocalizedName {
    pub fn contains(&self, search_term: &str) -> bool {
        self.deDE.to_lowercase().contains(search_term)
            || self.enUS.to_lowercase().contains(search_term)
            || self.esES.to_lowercase().contains(search_term)
            || self.esMX.to_lowercase().contains(search_term)
            || self.frFR.to_lowercase().contains(search_term)
            || self.itIT.to_lowercase().contains(search_term)
            || self.jaJP.contains(search_term)
            || self.koKR.contains(search_term)
            || self.plPL.to_lowercase().contains(search_term)
            || self.ptBR.to_lowercase().contains(search_term)
            || self.ruRU.to_lowercase().contains(search_term)
            || self.thTH.contains(search_term)
            || self.zhCN.as_ref().is_some_and(|s| s.contains(search_term))
            || self.zhTW.contains(search_term)
    }
}
impl Localize for LocalizedName {
    fn in_locale(&self, locale: Locale) -> &str {
        match locale {
            Locale::deDE => self.deDE.as_str(),
            Locale::enUS => self.enUS.as_str(),
            Locale::esES => self.esES.as_str(),
            Locale::esMX => self.esMX.as_str(),
            Locale::frFR => self.frFR.as_str(),
            Locale::itIT => self.itIT.as_str(),
            Locale::jaJP => self.jaJP.as_str(),
            Locale::koKR => self.koKR.as_str(),
            Locale::plPL => self.plPL.as_str(),
            Locale::ptBR => self.ptBR.as_str(),
            Locale::ruRU => self.ruRU.as_str(),
            Locale::thTH => self.thTH.as_str(),
            Locale::zhCN => self.zhCN.as_deref().unwrap_or(self.zhTW.as_str()),
            Locale::zhTW => self.zhTW.as_str(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Details<ID> {
    // ID is the id type. usually u8 but it is usize for factions.
    // probably should just make it usize all over.
    pub id: ID,
    #[serde(with = "either::serde_untagged")]
    name: Either<LocalizedName, CompactString>,
}
impl<ID> Details<ID> {
    pub fn contains(&self, search_term: &str) -> bool {
        match self.name.as_ref() {
            Left(ln) => ln.deDE.eq_ignore_ascii_case(search_term)
                    || ln.enUS.eq_ignore_ascii_case(search_term)
                    || ln.esES.eq_ignore_ascii_case(search_term)
                    || ln.esMX.eq_ignore_ascii_case(search_term)
                    || ln.frFR.eq_ignore_ascii_case(search_term)
                    || ln.itIT.eq_ignore_ascii_case(search_term)
                    || ln.jaJP.eq(search_term)
                    || ln.koKR.eq(search_term)
                    || ln.plPL.eq_ignore_ascii_case(search_term)
                    || ln.ptBR.eq_ignore_ascii_case(search_term)
                    || ln.ruRU.eq_ignore_ascii_case(search_term)
                    || ln.thTH.eq(search_term)
                    || ln.zhCN.as_ref().is_some_and(|s| s.eq(search_term))
                    || ln.zhTW.eq(search_term),
            Right(s) => s.eq_ignore_ascii_case(search_term),
        }
    }
    pub fn name(&self, locale: Locale) -> CompactString {
        self.name.clone().right_or_else(|ln| ln.in_locale(locale).to_compact_string())
    }
}

static METADATA: RwLock<Option<(Metadata, Instant)>> = RwLock::new(None);
const REFRESH_RATE: Duration = Duration::from_secs(86400); // a day

fn internal_get_metadata() -> Metadata {
    AGENT.get("https://us.api.blizzard.com/hearthstone/metadata")
        .header("Authorization", format!("Bearer {}", get_access_token()))
        .call()
        .and_then(|mut res| res.body_mut().read_json::<Metadata>())
        .unwrap_or_default()
}

pub(crate) fn get_metadata() -> MappedRwLockReadGuard<'static, Metadata> {
    let last_update = METADATA.read().as_ref().map(|o| o.1);
    if last_update.is_none_or(|t| t.elapsed() >= REFRESH_RATE) {
        _ = METADATA.write().insert((internal_get_metadata(), Instant::now()));
    }

    RwLockReadGuard::map(METADATA.read(), |c| &c.as_ref().unwrap().0)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Set {
    id: usize,
    name: LocalizedName,
    alias_set_ids: Option<Vec<usize>>,
}
impl Localize for Set {
    fn in_locale(&self, locale: Locale) -> &str {
        self.name.in_locale(locale)
    }
}

pub(crate) fn get_set_by_id(id: usize, locale: Locale) -> CompactString {
    get_metadata()
        .sets
        .iter()
        .find(|s| s.id == id || s.alias_set_ids.iter().flatten().contains(&id))
        .map_or_else(
            || match id {
                1453 => locale.battlegrounds().into(),
                7 => "Hero Portraits".into(), // Should localize this
                1586 => "Mercenaries".into(), // and this.
                _ => format_compact!("Set {id}"),
            },
            |s| s.in_locale(locale).to_compact_string(),
        )
}

#[derive(EnumSetType, Hash, Deserialize)]
#[serde(rename_all = "lowercase")] // for Firestone's API.
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
}
impl Localize for Class {
    fn in_locale(&self, locale: Locale) -> impl Display {
        get_metadata()
            .classes
            .iter()
            .find(|det| Self::try_from(det.id).is_ok_and(|c| c == *self))
            .map_or("UNKNOWN".into(), |det| det.name(locale))
    }
}
impl TryFrom<u8> for Class {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
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
            _ => anyhow::bail!("Not a valid class (yet?)"),
        })
    }
}
impl FromStr for Class {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DK" | "Dk" | "dk" => Ok(Self::DeathKnight),
            "DH" | "Dh" | "dh" => Ok(Self::DemonHunter),
            "DR" | "Dr" | "dr" => Ok(Self::Druid),
            // "EV" | "Ev" | "ev" => Ok(Self::Evoker),
            "HU" | "Hu" | "hu" => Ok(Self::Hunter),
            "MA" | "Ma" | "ma" => Ok(Self::Mage),
            // "MO" | "Mo" | "mo" => Ok(Self::Monk),
            "PA" | "Pa" | "pa" => Ok(Self::Paladin),
            "PR" | "Pr" | "pr" => Ok(Self::Priest),
            "RO" | "Ro" | "ro" => Ok(Self::Rogue),
            "SH" | "Sh" | "sh" => Ok(Self::Shaman),
            "WL" | "Wl" | "wl" | "WK" | "Wk" | "wk" => Ok(Self::Warlock),
            "WR" | "Wr" | "wr" => Ok(Self::Warrior),
            _ => get_metadata()
                .classes
                .iter()
                .find(|det| det.contains(s))
                .and_then(|det| Self::try_from(det.id).ok())
                .ok_or_else(|| anyhow::anyhow!("Not a valid class (yet?)")),
        }
    }
}
impl Class {
    #[must_use]
    pub const fn color(self) -> (u8, u8, u8) {
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
        }
    }
}

impl Localize for EnumSet<Class> {
    fn in_locale(&self, locale: Locale) -> impl Display {
        self.into_iter()
            .map(|c| c.in_locale(locale).to_compact_string())
            .reduce(|a, b| format_compact!("{a}/{b}"))
            .unwrap_or_else(|| get_metadata()
                .classes
                .iter()
                .find(|det| det.id == 12) // Neutral
                .expect("Neutral (12) always exists")
                .name(locale)
            )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Faction(pub usize);

impl Localize for Faction {
    fn in_locale(&self, locale: Locale) -> impl Display {
        get_metadata()
            .factions
            .iter()
            .find(|det| self.0 == det.id)
            .map(|det| det.name(locale))
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Rarity { Legendary, Epic, Rare, Common, Free, Noncollectible }

impl Localize for Rarity {
    fn in_locale(&self, locale: Locale) -> impl Display {
        let text: CompactString = get_metadata()
            .rarities
            .iter()
            .find(|det| *self == Self::from(det.id))
            .map(|det| det.name(locale))
            .unwrap_or_default();

        match self {
            Self::Common | Self::Free => text.to_lowercase().white(),
            Self::Rare => text.to_lowercase().blue(),
            Self::Epic => text.to_lowercase().purple(),
            Self::Legendary => text.to_uppercase().bright_yellow().bold(),
            Self::Noncollectible => "Noncollectible".clear(),
        }
        .italic()
    }
}
impl From<u8> for Rarity {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Common,
            2 => Self::Free,
            3 => Self::Rare,
            4 => Self::Epic,
            5 => Self::Legendary,
            _ => Self::Noncollectible, // Fine default state.
        }
    }
}
impl Rarity {
    #[must_use]
    pub const fn color(&self) -> (u8, u8, u8) {
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpellSchool {
    Arcane, Fire,   Frost, Nature,
    Holy,   Shadow, Fel,

    // BG Schools. Show up in tokens search.
    Spellcraft, Tavern,
    
    // BG Trinkets. They're grouped with Spell Schools in the API.
    Greater, Lesser,
}
impl Localize for SpellSchool {
    fn in_locale(&self, locale: Locale) -> impl Display {
        get_metadata()
            .spell_schools
            .iter()
            .find(|det| *self == Self::from(det.id))
            .map_or("UNKNOWN".into(), |det| det.name(locale))
    }
}
impl From<u8> for SpellSchool {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Arcane,
            2 => Self::Fire,
            3 => Self::Frost,
            4 => Self::Nature,
            5 => Self::Holy,
            6 => Self::Shadow,
            7 => Self::Fel,
            // what is 8?
            10 => Self::Spellcraft,
            11 => Self::Lesser,
            12 => Self::Greater,
            _ => Self::Tavern, // 9
        }
    }
}

// All minion types in the game, including for Mercenaries, are listed.
// This is to futureproof adding any of them to Standard in the future.
#[derive(EnumSetType)]
pub enum MinionType {
    BloodElf, Draenei,   Dwarf,  Gnome,
    Human,    NightElf,  Orc,    Tauren,
    Troll,    Undead,    Murloc, Demon,
    Mech,     Elemental, Beast,  Totem,
    Pirate,   Dragon,    All,    Quilboar,
    HalfOrc,  Naga,      OldGod, Pandaren,
    Gronn, // Gruul is cool.
}
impl Localize for MinionType {
    fn in_locale(&self, locale: Locale) -> impl Display {
        get_metadata()
            .minion_types
            .iter()
            .find(|det| Self::try_from(det.id).is_ok_and(|s| s == *self))
            .map_or("UNKNOWN".into(), |det| det.name(locale))
    }
}
impl TryFrom<u8> for MinionType {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::BloodElf,
            2 => Self::Draenei,
            3 => Self::Dwarf,
            4 => Self::Gnome,
            6 => Self::Human,
            7 => Self::NightElf,
            8 => Self::Orc,
            9 => Self::Tauren,
            10 => Self::Troll,
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
            88 => Self::HalfOrc,
            92 => Self::Naga,
            93 => Self::OldGod,
            94 => Self::Pandaren,
            95 => Self::Gronn,
            _ => anyhow::bail!("Not a valid minion type ID."),
        })
    }
}
impl FromStr for MinionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        get_metadata()
            .minion_types
            .iter()
            .find(|det| det.contains(s))
            .and_then(|det| Self::try_from(det.id).ok())
            .ok_or_else(|| anyhow::anyhow!("Not a valid minion type (yet?)"))
    }
}

impl Localize for EnumSet<MinionType> {
    fn in_locale(&self, locale: Locale) -> impl Display {
        self.into_iter()
            .map(|c| c.in_locale(locale).to_compact_string())
            .reduce(|a, b| format_compact!("{a}/{b}"))
            .unwrap_or_else(|| get_metadata()
                .types
                .iter()
                .find(|det| det.id == 4) // 4 for Minion
                .expect("Minion (4) always exists")
                .name(locale)
            )
    }
}

#[derive(Clone, Copy, Deserialize)]
pub struct RuneCost { blood: u8, frost: u8, unholy: u8 }

impl Display for RuneCost {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        (0..self.blood)
            .map(|_| "B".red())
            .chain((0..self.frost).map(|_| "F".blue()))
            .chain((0..self.unholy).map(|_| "U".green()))
            .try_for_each(|c| write!(f, "{c}"))
    }
}

#[derive(Clone, Copy)]
pub enum CardType {
    Hero { armor: u8 },
    Minion { attack: u8, health: u8, minion_types: EnumSet<MinionType> },
    Spell { school: Option<SpellSchool> },
    Weapon { attack: u8, durability: u8 },
    Location { durability: u8 },
    HeroPower,
    Unknown,
}
impl Localize for CardType {
    fn in_locale(&self, locale: Locale) -> impl Display {
        // Wizardry. Implement an InnerType that implements Display with all its
        // ergonomics, and return it!!
        struct Inner<'a>(&'a CardType, Locale);

        impl Display for Inner<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                // Not sure what f.alternate() is used for right now.
                let colon = if f.alternate() { ":" } else { "" };

                let get_type =
                    |i| get_metadata().types.iter().find(|det| det.id == i).unwrap().name(self.1);

                match self.0 {
                    CardType::Hero { armor } => {
                        let hero = get_type(3); // 3 for Hero
                        write!(f, "{hero} [{armor}]{colon}")
                    }
                    CardType::Minion { attack, health, minion_types } => {
                        let blurp = minion_types.in_locale(self.1);
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
                        write!(f, "/{durability} {location}{colon}")
                    }
                    CardType::HeroPower => {
                        // 10 for Hero Power. these numbers should be in the type itself tbh
                        let heropower = get_type(10);
                        write!(f, "{heropower}{colon}")
                    }
                    CardType::Unknown => write!(f, "UNKNOWN{colon}"),
                }
            }
        }

        Inner(self, locale)
    }
}
