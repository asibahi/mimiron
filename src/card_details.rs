use colored::Colorize;
use itertools::Itertools;
use serde::Deserialize;
use std::{collections::HashSet, fmt::Display, str::FromStr};

#[allow(dead_code)]
#[derive(PartialEq, Eq, Hash, Clone, Deserialize)]
#[serde(from = "ClassData")]
pub enum Class {
    DeathKnight,
    DemonHunter,
    Druid,
    Evoker,
    Hunter,
    Mage,
    Monk,
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            // colors from D0nkey's site.
            Class::DeathKnight => "DeathKnight",
            Class::DemonHunter => "DemonHunter",
            Class::Druid => "Druid",
            Class::Evoker => "Evoker",
            Class::Hunter => "Hunter",
            Class::Mage => "Mage",
            Class::Monk => "Monk",
            Class::Paladin => "Paladin",
            Class::Priest => "Priest",
            Class::Rogue => "Rogue",
            Class::Shaman => "Shaman",
            Class::Warlock => "Warlock",
            Class::Warrior => "Warrior",
            Class::Neutral => "Neutral",
            Class::Unknown => "UNKNOWN",
        };
        write!(f, "{s}")
    }
}
impl From<u8> for Class {
    fn from(value: u8) -> Self {
        match value {
            1 => Class::DeathKnight,
            14 => Class::DemonHunter,
            2 => Class::Druid,
            // placeholder => Class::Evoker,
            3 => Class::Hunter,
            4 => Class::Mage,
            // placeholder => Class::Monk,
            5 => Class::Paladin,
            6 => Class::Priest,
            7 => Class::Rogue,
            8 => Class::Shaman,
            9 => Class::Warlock,
            10 => Class::Warrior,
            12 => Class::Neutral,
            _ => Class::Unknown,
        }
    }
}
impl From<ClassData> for Class {
    fn from(value: ClassData) -> Self {
        value.id.into()
    }
}

#[derive(Deserialize)]
pub struct ClassData {
    id: u8,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum Rarity {
    Legendary,
    Epic,
    Rare,
    Common,
    Free,
    Unknown,
}
impl Display for Rarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = match self {
            Rarity::Common => "common".white(),
            Rarity::Free => "free".white(),
            Rarity::Rare => "rare".blue(),
            Rarity::Epic => "epic".purple(),
            Rarity::Legendary => "LEGENDARY".bright_yellow().bold(),
            Rarity::Unknown => "UNKNOWN".clear(),
        }
        .italic();
        write!(f, "{r}")
    }
}
impl From<u8> for Rarity {
    fn from(value: u8) -> Self {
        match value {
            1 => Rarity::Common,
            2 => Rarity::Free,
            3 => Rarity::Rare,
            4 => Rarity::Epic,
            5 => Rarity::Legendary,
            _ => Rarity::Unknown,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SpellSchool::Arcane => "Arcane",
            SpellSchool::Fire => "Fire",
            SpellSchool::Frost => "Frost",
            SpellSchool::Nature => "Nature",
            SpellSchool::Holy => "Holy",
            SpellSchool::Shadow => "Shadow",
            SpellSchool::Fel => "Fel",
            SpellSchool::Unknown => "UNKNOWN",
        };

        write!(f, "{s}")
    }
}
impl From<u8> for SpellSchool {
    fn from(value: u8) -> Self {
        match value {
            1 => SpellSchool::Arcane,
            2 => SpellSchool::Fire,
            3 => SpellSchool::Frost,
            4 => SpellSchool::Nature,
            5 => SpellSchool::Holy,
            6 => SpellSchool::Shadow,
            7 => SpellSchool::Fel,
            _ => SpellSchool::Unknown,
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = match self {
            MinionType::Undead => "Undead",
            MinionType::Murloc => "Murloc",
            MinionType::Demon => "Demon",
            MinionType::Mech => "Mech",
            MinionType::Elemental => "Elemental",
            MinionType::Beast => "Beast",
            MinionType::Totem => "Totem",
            MinionType::Pirate => "Pirate",
            MinionType::Dragon => "Dragon",
            MinionType::All => "Amalgam",
            MinionType::Quilboar => "Quilboar",
            MinionType::Naga => "Naga",
            MinionType::Unknown => "UNKNOWN",
        };

        write!(f, "{t}")
    }
}
impl From<u8> for MinionType {
    //   type Error = anyhow::Error;

    fn from(value: u8) -> Self {
        match value {
            11 => MinionType::Undead,
            14 => MinionType::Murloc,
            15 => MinionType::Demon,
            17 => MinionType::Mech,
            18 => MinionType::Elemental,
            20 => MinionType::Beast,
            21 => MinionType::Totem,
            23 => MinionType::Pirate,
            24 => MinionType::Dragon,
            26 => MinionType::All,
            43 => MinionType::Quilboar,
            92 => MinionType::Naga,
            _ => MinionType::Unknown,
        }
    }
}
impl FromStr for MinionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = match s.to_lowercase().as_ref() {
            "undead" => MinionType::Undead,
            "murloc" => MinionType::Murloc,
            "demon" => MinionType::Demon,
            "mech" => MinionType::Mech,
            "elemental" => MinionType::Elemental,
            "beast" => MinionType::Beast,
            "totem" => MinionType::Totem,
            "pirate" => MinionType::Pirate,
            "dragon" => MinionType::Dragon,
            "amalgam" => MinionType::All,
            "quilboar" => MinionType::Quilboar,
            "naga" => MinionType::Naga,
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
        let bl = "B".repeat(self.blood as usize);
        let fr = "F".repeat(self.frost as usize);
        let un = "U".repeat(self.unholy as usize);
        write!(f, "{bl}{fr}{un}")
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CardType::Hero { armor } => write!(f, "Hero card with {armor} armor"),
            CardType::Minion {
                attack,
                health,
                minion_types,
            } => {
                if minion_types.is_empty() {
                    write!(f, "{attack}/{health} minion")
                } else {
                    let types = minion_types.iter().join("/");
                    write!(f, "{attack}/{health} {types}")
                }
            }
            CardType::Spell { school } => match school {
                Some(s) => write!(f, "{s} spell"),
                None => write!(f, "spell"),
            },
            CardType::Weapon { attack, durability } => write!(f, "{attack}/{durability} weapon"),
            CardType::Location { durability } => write!(f, "{durability} durability location"),
            CardType::Unknown => write!(f, "UNKNOWN"),
        }
    }
}
