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

static SETS: Lazy<Vec<Set>> = Lazy::new(|| {
    AGENT
        .get("https://us.api.blizzard.com/hearthstone/metadata/sets")
        .query("locale", &Locale::enUS.to_string())
        .query("access_token", &get_access_token())
        .call()
        .and_then(|res| Ok(res.into_json::<Vec<Set>>()?))
        .unwrap_or_default()
});

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Set {
    id: usize,
    name: String,
    alias_set_ids: Option<Vec<usize>>,
}

pub(crate) fn get_set_by_id(id: usize) -> String {
    let set = SETS.iter().find(|s| {
        s.id == id
            || s.alias_set_ids
                .as_ref()
                .is_some_and(|aliases| aliases.contains(&id))
    });

    set.map_or_else(|| format!("Set {id}"), |s| s.name.clone())
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
impl Display for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::DeathKnight => "Death Knight",
            Self::DemonHunter => "Demon Hunter",
            Self::Druid => "Druid",
            // Self::Evoker => "Evoker",
            Self::Hunter => "Hunter",
            Self::Mage => "Mage",
            // Self::Monk => "Monk",
            Self::Paladin => "Paladin",
            Self::Priest => "Priest",
            Self::Rogue => "Rogue",
            Self::Shaman => "Shaman",
            Self::Warlock => "Warlock",
            Self::Warrior => "Warrior",
            Self::Neutral => "Neutral",
            Self::Unknown => "UNKNOWN",
        };
        write!(f, "{s}")
    }
}
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

#[derive(Clone)]
pub enum Rarity {
    Legendary,
    Epic,
    Rare,
    Common,
    Free,
    Noncollectible,
}
impl Display for Rarity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let r = match self {
            Self::Common => "common".white(),
            Self::Free => "free".white(),
            Self::Rare => "rare".blue(),
            Self::Epic => "epic".purple(),
            Self::Legendary => "LEGENDARY".bright_yellow().bold(),
            Self::Noncollectible => "Noncollectible".clear(),
        }
        .italic();
        write!(f, "{r}")
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

#[derive(Clone)]
pub enum SpellSchool {
    Arcane,
    Fire,
    Frost,
    Nature,
    Holy,
    Shadow,
    Fel,
    Unknown,
}
impl Display for SpellSchool {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Arcane => "Arcane",
            Self::Fire => "Fire",
            Self::Frost => "Frost",
            Self::Nature => "Nature",
            Self::Holy => "Holy",
            Self::Shadow => "Shadow",
            Self::Fel => "Fel",
            Self::Unknown => "UNKNOWN",
        };

        write!(f, "{s}")
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
            _ => Self::Unknown,
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
    Unknown,
}
impl MinionType {
    pub fn in_locale(&self, locale: &Locale) -> String {
        let s = match self {
            Self::Undead => match locale {
                Locale::deDE => "Untot",
                Locale::enUS => "Undead",
                Locale::esES | Locale::esMX => "No-muerto",
                Locale::frFR => "Mort-vivant",
                Locale::itIT => "Non Morto",
                Locale::jaJP => "アンデッド",
                Locale::koKR => "언데드",
                Locale::plPL => "Nieumarły",
                Locale::ptBR => "Morto-vivo",
                Locale::ruRU => "Нежить",
                Locale::thTH => "อันเดด",
                Locale::zhCN => "亡灵",
                Locale::zhTW => "不死族",
            },
            Self::Murloc => match locale {
                Locale::deDE => "Murloc",
                Locale::enUS => "Murloc",
                Locale::esES | Locale::esMX => "Múrloc",
                Locale::frFR => "Murloc",
                Locale::itIT => "Murloc",
                Locale::jaJP => "マーロック",
                Locale::koKR => "멀록",
                Locale::plPL => "Murlok",
                Locale::ptBR => "Murloc",
                Locale::ruRU => "Мурлок",
                Locale::thTH => "เมอร์ล็อค",
                Locale::zhCN => "鱼人",
                Locale::zhTW => "魚人",
            },
            Self::Demon => match locale {
                Locale::deDE => "Dämon",
                Locale::enUS => "Demon",
                Locale::esES | Locale::esMX => "Demonio",
                Locale::frFR => "Démon",
                Locale::itIT => "Demone",
                Locale::jaJP => "悪魔",
                Locale::koKR => "악마",
                Locale::plPL => "Demon",
                Locale::ptBR => "Demônio",
                Locale::ruRU => "Демон",
                Locale::thTH => "ปีศาจ",
                Locale::zhCN => "恶魔",
                Locale::zhTW => "惡魔",
            },
            Self::Mech => match locale {
                Locale::deDE => "Mech",
                Locale::enUS => "Mech",
                Locale::esES => "Robot",
                Locale::esMX => "Meca",
                Locale::frFR => "Méca",
                Locale::itIT => "Robot",
                Locale::jaJP => "メカ",
                Locale::koKR => "기계",
                Locale::plPL => "Mech",
                Locale::ptBR => "Mecanoide",
                Locale::ruRU => "Механизм",
                Locale::thTH => "เครื่องจักร",
                Locale::zhCN => "机械",
                Locale::zhTW => "機械",
            },
            Self::Elemental => match locale {
                Locale::deDE => "Elementar",
                Locale::enUS => "Elemental",
                Locale::esES | Locale::esMX => "Elemental",
                Locale::frFR => "Élémentaire",
                Locale::itIT => "Elementale",
                Locale::jaJP => "エレメンタル",
                Locale::koKR => "정령",
                Locale::plPL => "Żywiołak",
                Locale::ptBR => "Elemental",
                Locale::ruRU => "Элементаль",
                Locale::thTH => "วิญญาณธาตุ",
                Locale::zhCN => "元素",
                Locale::zhTW => "元素",
            },
            Self::Beast => match locale {
                Locale::deDE => "Wildtier",
                Locale::enUS => "Beast",
                Locale::esES | Locale::esMX => "Bestia",
                Locale::frFR => "Bête",
                Locale::itIT => "Bestia",
                Locale::jaJP => "獣",
                Locale::koKR => "야수",
                Locale::plPL => "Bestia",
                Locale::ptBR => "Fera",
                Locale::ruRU => "Зверь",
                Locale::thTH => "สัตว์",
                Locale::zhCN => "野兽",
                Locale::zhTW => "野獸",
            },
            Self::Totem => match locale {
                Locale::deDE => "Totem",
                Locale::enUS => "Totem",
                Locale::esES | Locale::esMX => "Tótem",
                Locale::frFR => "Totem",
                Locale::itIT => "Totem",
                Locale::jaJP => "トーテム",
                Locale::koKR => "토템",
                Locale::plPL => "Totem",
                Locale::ptBR => "Totem",
                Locale::ruRU => "Тотем",
                Locale::thTH => "โทเท็ม",
                Locale::zhCN => "图腾",
                Locale::zhTW => "圖騰",
            },
            Self::Pirate => match locale {
                Locale::deDE => "Pirat",
                Locale::enUS => "Pirate",
                Locale::esES | Locale::esMX => "Pirata",
                Locale::frFR => "Pirate",
                Locale::itIT => "Pirata",
                Locale::jaJP => "海賊",
                Locale::koKR => "해적",
                Locale::plPL => "Pirat",
                Locale::ptBR => "Pirata",
                Locale::ruRU => "Пират",
                Locale::thTH => "โจรสลัด",
                Locale::zhCN => "海盗",
                Locale::zhTW => "海盜",
            },
            Self::Dragon => match locale {
                Locale::deDE => "Drache",
                Locale::enUS => "Dragon",
                Locale::esES | Locale::esMX => "Dragón",
                Locale::frFR => "Dragon",
                Locale::itIT => "Drago",
                Locale::jaJP => "ドラゴン",
                Locale::koKR => "용족",
                Locale::plPL => "Smok",
                Locale::ptBR => "Dragão",
                Locale::ruRU => "Дракон",
                Locale::thTH => "มังกร",
                Locale::zhCN => "龙",
                Locale::zhTW => "龍類",
            },
            Self::All => match locale {
                Locale::deDE => "Alle",
                Locale::enUS => "All",
                Locale::esES => "Todos",
                Locale::esMX => "Todas",
                Locale::frFR => "Tout",
                Locale::itIT => "Tutti",
                Locale::jaJP => "全て",
                Locale::koKR => "모두",
                Locale::plPL => "Wszystkie",
                Locale::ptBR => "Tudo",
                Locale::ruRU => "Все",
                Locale::thTH => "ทุกอย่าง",
                Locale::zhCN => "全部",
                Locale::zhTW => "全部",
            },
            Self::Quilboar => match locale {
                Locale::deDE => "Stacheleber",
                Locale::enUS => "Quilboar",
                Locale::esES => "Jabaespín",
                Locale::esMX => "Jabaespín",
                Locale::frFR => "Huran",
                Locale::itIT => "Verrospino",
                Locale::jaJP => "キルボア",
                Locale::koKR => "가시멧돼지",
                Locale::plPL => "Kolcozwierz",
                Locale::ptBR => "Javatusco",
                Locale::ruRU => "Свинобраз",
                Locale::thTH => "ควิลบอร์",
                Locale::zhCN => "野猪人",
                Locale::zhTW => "野豬人",
            },
            Self::Naga => match locale {
                Locale::deDE
                | Locale::enUS
                | Locale::esES
                | Locale::esMX
                | Locale::frFR
                | Locale::itIT
                | Locale::plPL
                | Locale::ptBR => "Naga",
                Locale::jaJP => "ナーガ",
                Locale::koKR => "나가",
                Locale::ruRU => "Нага",
                Locale::thTH => "นากา",
                Locale::zhCN => "纳迦",
                Locale::zhTW => "納迦",
            },
            Self::Unknown => "UKNOWN",
        };

        s.into()
    }
}
impl Display for MinionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let t = match self {
            Self::Undead => "Undead",
            Self::Murloc => "Murloc",
            Self::Demon => "Demon",
            Self::Mech => "Mech",
            Self::Elemental => "Elemental",
            Self::Beast => "Beast",
            Self::Totem => "Totem",
            Self::Pirate => "Pirate",
            Self::Dragon => "Dragon",
            Self::All => "Amalgam",
            Self::Quilboar => "Quilboar",
            Self::Naga => "Naga",
            Self::Unknown => "UNKNOWN",
        };

        write!(f, "{t}")
    }
}
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
            92 => Self::Naga,
            _ => Self::Unknown,
        }
    }
}
impl FromStr for MinionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = match s.to_lowercase().as_ref() {
            "undead" => Self::Undead,
            "murloc" => Self::Murloc,
            "demon" => Self::Demon,
            "mech" => Self::Mech,
            "elemental" => Self::Elemental,
            "beast" => Self::Beast,
            "totem" => Self::Totem,
            "pirate" => Self::Pirate,
            "dragon" => Self::Dragon,
            "amalgam" => Self::All,
            "quilboar" => Self::Quilboar,
            "naga" => Self::Naga,
            _ => return Err(anyhow::anyhow!("Not a valid minion type (yet?)")),
        };
        Ok(t)
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
impl Display for CardType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // f.alternate() is used for text boxes on images. Regular mode for console output.
        let colon = if f.alternate() { ":" } else { "" };
        match self {
            Self::Hero { armor } => write!(f, "Hero card with {armor} armor{colon}"),
            Self::Minion {
                attack,
                health,
                minion_types,
            } => {
                let types = minion_types.iter().join("/");
                write!(
                    f,
                    "{attack}/{health} {}{colon}",
                    if types.is_empty() { "minion" } else { &types }
                )
            }
            Self::Spell { school } => match school {
                Some(s) => write!(f, "{s} spell{colon}"),
                None if f.alternate() => write!(f, "Spell:"),
                None => write!(f, "spell"),
            },
            Self::Weapon { attack, durability } => write!(f, "{attack}/{durability} weapon{colon}"),
            Self::Location { durability } => write!(f, "{durability} durability location{colon}"),
            Self::HeroPower => write!(f, "Hero Power{colon}"),
            Self::Unknown => write!(f, "UNKNOWN{colon}"),
        }
    }
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
