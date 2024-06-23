use anyhow::anyhow;
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

#[allow(non_camel_case_types, dead_code)]
#[derive(Clone, Copy, Default)]
pub enum Locale {
    deDE,
    #[default]
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
impl Locale {
    // Inner Functions that note global terms that might be used in multiple places
    pub(crate) fn battlegrounds(self) -> &'static str {
        match self {
            Locale::deDE => "Schlachtfeld",
            Locale::frFR => "Champs de bataille",
            Locale::itIT => "Battaglia",
            Locale::jaJP => "バトルグラウンド",
            Locale::koKR => "전장",
            Locale::plPL => "Ustawka",
            Locale::ptBR => "Campos de Batalha",
            Locale::ruRU => "Поля сражений",
            Locale::enUS
            | Locale::esES
            | Locale::esMX
            | Locale::thTH
            | Locale::zhCN
            | Locale::zhTW => "Battlegrounds",
        }
    }

    pub(crate) fn quest(self) -> &'static str {
        match self {
            Locale::deDE | Locale::enUS => "Quest",
            Locale::esES | Locale::esMX => "Misión",
            Locale::frFR => "Quête",
            Locale::itIT => "Missione",
            Locale::jaJP => "クエスト",
            Locale::koKR => "퀘스트",
            Locale::plPL => "Zadanie",
            Locale::ptBR => "Missão",
            Locale::ruRU => "Задача",
            Locale::thTH => "เควสต์",
            Locale::zhCN => "任务",
            Locale::zhTW => "任務",
        }
    }
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
impl FromStr for Locale {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();

        if s.starts_with("de") {
            Ok(Self::deDE)
        } else if s.starts_with("en") {
            Ok(Self::enUS)
        } else if s == "esmx"
            || s == "esla"
            || s == "es_mx"
            || s == "es_la"
            || s.starts_with("mx")
            || s.starts_with("la")
        {
            Ok(Self::esMX)
        } else if s.starts_with("es") {
            Ok(Self::esES)
        } else if s.starts_with("fr") {
            Ok(Self::frFR)
        } else if s.starts_with("it") {
            Ok(Self::itIT)
        } else if s.starts_with("ja") || s.starts_with("jp") {
            Ok(Self::jaJP)
        } else if s.starts_with("ko") || s.starts_with("kr") {
            Ok(Self::koKR)
        } else if s.starts_with("pl") {
            Ok(Self::plPL)
        } else if s.starts_with("pt") || s.starts_with("br") {
            Ok(Self::ptBR)
        } else if s.starts_with("ru") {
            Ok(Self::ruRU)
        } else if s.starts_with("th") {
            Ok(Self::thTH)
        } else if s == "zhcn" || s == "zh_cn" {
            Ok(Self::zhCN)
        } else if s.starts_with("zh") {
            Ok(Self::zhTW)
        } else {
            Err(anyhow!("Could not parse locale."))
        }
    }
}

pub trait Localize {
    fn in_locale(&self, locale: Locale) -> impl Display;

    fn in_en_us(&self) -> impl Display {
        self.in_locale(Locale::enUS)
    }
}
