use colored::Colorize;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
    str::FromStr,
};

#[allow(dead_code)]
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
            Self::DeathKnight => "DeathKnight",
            Self::DemonHunter => "DemonHunter",
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
    pub(crate) fn color(&self) -> (u8, u8, u8) {
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
            _ => (169, 169, 169),
        }
    }
}

#[derive(Deserialize)]
struct ClassData {
    // slug: String,
    id: u8,
    // name: String,
}

#[derive(Clone)]
pub enum Rarity {
    Legendary,
    Epic,
    Rare,
    Common,
    Free,
    Unknown,
}
impl Display for Rarity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let r = match self {
            Self::Common => "common".white(),
            Self::Free => "free".white(),
            Self::Rare => "rare".blue(),
            Self::Epic => "epic".purple(),
            Self::Legendary => "LEGENDARY".bright_yellow().bold(),
            Self::Unknown => "UNKNOWN".clear(),
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
            _ => Self::Unknown,
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
            Self::Unknown => write!(f, "UNKNOWN{colon}"),
        }
    }
}
