use crate::{
    AGENT,
    card::Card,
    card_details::{CardType, Class, Details},
    get_access_token,
    hearth_sim::validate_id,
    localization::{Locale, Localize},
};
use anyhow::{Result, anyhow};
use colored::Colorize;
use compact_str::{CompactString, ToCompactString, format_compact};
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{Display, Write},
    ops::Not,
    str::FromStr,
};

pub use crate::deck_image::ImageOptions;

#[derive(Clone, Default, Deserialize, Debug, PartialEq)]
#[serde(from = "String")]
pub enum Format {
    #[default]
    Standard,
    Wild,
    Classic,
    Twist,
    Custom(CompactString),
}

impl Display for Format {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Standard => write!(f, "Standard"),
            Self::Wild => write!(f, "Wild"),
            Self::Classic => write!(f, "Classic"),
            Self::Twist => write!(f, "Twist"),
            Self::Custom(fmt) => write!(f, "{fmt}"),
        }
    }
}
impl FromStr for Format {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.to_owned().into())
    }
}
impl From<String> for Format {
    fn from(mut value: String) -> Self {
        value.make_ascii_lowercase();
        match value.as_str() {
            "wild" | "wld" | "w" => Self::Wild,
            "standard" | "std" | "s" => Self::Standard,
            "twist" | "t" => Self::Twist,
            "classic" | "c" => Self::Classic,
            _ => Self::Custom(value.into()),
        }
    }
}
impl TryFrom<u8> for Format {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::Wild,
            2 => Self::Standard,
            3 => Self::Classic,
            4 => Self::Twist,
            _ => anyhow::bail!("Not a valid format ID."),
        })
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sideboard {
    pub sideboard_card: Card,
    pub cards_in_sideboard: Vec<Card>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeckData {
    deck_code: CompactString,
    format: Format,
    hero: Card,
    class: Details<u8>,
    cards: Vec<Card>,

    #[serde(default)]
    sideboard_cards: Vec<Sideboard>,

    #[serde(default)]
    invalid_card_ids: Vec<usize>,
}

#[derive(Clone, Deserialize)]
#[serde(from = "DeckData")]
pub struct Deck {
    pub title: CompactString,
    pub deck_code: CompactString,
    pub format: Format,
    pub class: Class,
    pub cards: Vec<Card>,
    pub sideboard_cards: Vec<Sideboard>,
    invalid_card_ids: Vec<usize>,
}
impl Deck {
    #[must_use]
    pub fn compare_with(
        &self,
        other: &Self,
    ) -> DeckDifference {
        use counter::Counter;

        let counter1 = self.cards.clone().into_iter().collect::<Counter<_>>();
        let counter2 = other.cards.clone().into_iter().collect::<Counter<_>>();

        let deck1_uniques = counter1.clone() - counter2.clone();

        DeckDifference {
            shared_cards: (counter1.clone() - deck1_uniques.clone()).into_map(),
            deck1_code: self.deck_code.clone(),
            deck1_uniques: deck1_uniques.into_map(),
            deck2_code: other.deck_code.clone(),
            deck2_uniques: (counter2 - counter1).into_map(),
        }
    }

    #[must_use]
    pub fn get_image(
        &self,
        opts: ImageOptions,
    ) -> image::RgbaImage {
        crate::deck_image::get(self, opts)
    }
}
impl From<DeckData> for Deck {
    fn from(value: DeckData) -> Self {
        Self {
            title: format_compact!(
                "{} - {}",
                value.hero.name,
                value.format.to_compact_string().to_uppercase()
            ),
            deck_code: value.deck_code,
            format: value.format,
            class: value.class.id.try_into().unwrap_or(Class::Mage),
            cards: value.cards,
            sideboard_cards: value.sideboard_cards,
            invalid_card_ids: value.invalid_card_ids,
        }
    }
}
impl Localize for Deck {
    fn in_locale(
        &self,
        locale: Locale,
    ) -> impl Display {
        let mut buffer = String::new();

        let code = &self.deck_code;

        writeln!(buffer, "\t{}", self.title.bold()).ok();

        let cards = self.cards.iter().fold(BTreeMap::new(), |mut map, card| {
            *map.entry(card).or_default() += 1;
            map
        });

        for (card, count) in cards {
            let count = format_count(count);
            writeln!(buffer, "{count:>4} {}", card.in_locale(locale)).ok();
        }

        {
            for sideboard in &self.sideboard_cards {
                writeln!(buffer, "Sideboard: {}", sideboard.sideboard_card.name).ok();

                let cards =
                    sideboard
                        .cards_in_sideboard
                        .iter()
                        .fold(BTreeMap::new(), |mut map, card| {
                            *map.entry(card).or_default() += 1;
                            map
                        });

                for (card, count) in cards {
                    let count = format_count(count);
                    writeln!(buffer, "{count:>4} {}", card.in_locale(locale)).ok();
                }
            }
        }

        write!(buffer, "{code}").ok();

        buffer
    }
}
pub struct DeckDifference {
    pub shared_cards: HashMap<Card, usize>,

    pub deck1_code: CompactString,
    pub deck1_uniques: HashMap<Card, usize>,

    pub deck2_code: CompactString,
    pub deck2_uniques: HashMap<Card, usize>,
}
impl Localize for DeckDifference {
    fn in_locale(
        &self,
        locale: Locale,
    ) -> impl Display {
        let mut f = String::new();
        for (card, count) in &self.shared_cards.iter().collect::<BTreeMap<_, _>>() {
            let count = format_count(**count);
            writeln!(f, "{count:>4} {}", card.in_locale(locale)).ok();
        }

        writeln!(f, "\n{}", self.deck1_code).ok();
        for (card, count) in &self.deck1_uniques.iter().collect::<BTreeMap<_, _>>() {
            let count = format_count(**count);
            writeln!(f, "{}{count:>3} {}", "+".green(), card.in_locale(locale)).ok();
        }

        writeln!(f, "\n{}", self.deck2_code).ok();
        for (card, count) in &self.deck2_uniques.iter().collect::<BTreeMap<_, _>>() {
            let count = format_count(**count);
            writeln!(f, "{}{count:>3} {}", "-".red(), card.in_locale(locale)).ok();
        }

        f
    }
}

pub struct LookupOptions<'s> {
    code: &'s str,
    locale: Locale,
    format: Option<&'s str>,
}

impl<'s> LookupOptions<'s> {
    #[must_use]
    pub const fn lookup(code: &'s str) -> Self {
        Self {
            code,
            locale: Locale::enUS,
            format: None,
        }
    }
    #[must_use]
    pub const fn with_locale(
        self,
        locale: Locale,
    ) -> Self {
        Self { locale, ..self }
    }
    #[must_use]
    pub const fn with_custom_format(
        self,
        format: Option<&'s str>,
    ) -> Self {
        Self { format, ..self }
    }
}

#[derive(Debug, PartialEq)]
struct RawCodeData {
    format: Format,
    hero: usize,
    cards: Vec<usize>,
    sideboard_cards: Vec<(usize, usize)>,
    deck_code: CompactString,
}

impl RawCodeData {
    fn from_code(code: &str) -> Option<Self> {
        // Deckstring encoding: https://hearthsim.info/docs/deckstrings/

        use base64::{
            alphabet,
            engine::{DecodePaddingMode, Engine as _, GeneralPurpose, GeneralPurposeConfig},
        };
        use nom::{
            Parser,
            branch::alt,
            bytes::{tag, take, take_while_m_n},
            combinator::{recognize, success},
            multi::length_count,
            number::u8,
            sequence::preceded,
        };

        const CONFIG: GeneralPurposeConfig =
            GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent);
        const ENGINE: GeneralPurpose = GeneralPurpose::new(&alphabet::STANDARD, CONFIG);

        #[allow(clippy::cast_possible_truncation)]
        fn varint<'a>() -> impl Parser<&'a [u8], Output = usize, Error = ()> {
            let is_ongoing = |b| b & 0x80 != 0;

            recognize(take_while_m_n(0, 8, is_ongoing).and(take(1u8))).map_opt(|p: &[u8]| {
                p.iter().enumerate().try_fold(0, |acc, (idx, byte)| {
                    ((*byte as usize) & 0x7F)
                        .checked_shl(idx as u32 * 7)
                        .map(|n| acc | n)
                })
            })
        }

        let decoded = ENGINE.decode(code).ok()?;
        let decoded = decoded.as_slice();

        #[cfg(debug_assertions)]
        {
            let raw_code = nom::combinator::iterator(decoded, varint())
                .fuse()
                .join(", ");
            tracing::info!(code, raw_code);
        }

        preceded(
            tag([0, 1].as_slice()),
            (
                // format
                u8().map(|f| f.try_into().unwrap_or_default()),
                // hero
                preceded(u8(), varint()),
                // cards
                (
                    // single cards
                    length_count(u8(), varint()),
                    // double cards
                    length_count(u8(), varint().map(|id| [id; 2])),
                    // n-count cards
                    length_count(
                        u8(),
                        (varint(), u8()).map(|(id, n)| [id].repeat(n as usize)),
                    ),
                )
                    .map(|(v1, v2, vn)| {
                        v1.into_iter()
                            .chain(v2.into_iter().flatten())
                            .chain(vn.into_iter().flatten())
                            .collect()
                    }),
                // sideboard
                alt((
                    preceded(
                        tag([1].as_slice()),
                        length_count(u8(), (varint(), varint())),
                    ),
                    success(Vec::new()),
                )),
            )
                .map(|(format, hero, cards, sideboard_cards)| Self {
                    format,
                    hero,
                    cards,
                    sideboard_cards,
                    deck_code: ENGINE.encode(decoded).into(), // Hearthstone requires base64 padding
                }),
        )
        .parse_complete(decoded)
        .map(|(_, rd)| rd)
        .ok()
    }
}

pub fn lookup(opts: LookupOptions<'_>) -> Result<Deck> {
    let code = &opts.code;
    /* For when someone pastes something like this:
     * ### Custom Shaman
     * # etc
     * #
     * AAECAfWfAwjy3QT0oAXmowXipAXFpQX9xAX0yAX00AUL1bIE4LUEssEExc4Exs4Euu0Eyu0EhaoFw9AFxNAFr9EFAAED2aAE/cQFr8MF/cQF0p4G/cQFAAA=
     * #
     * # To use this deck, copy it to your clipboard and create a new deck in Hearthstone
     */

    let raw_data = code
        // if it is a long code pasted from game or tracker
        .split_ascii_whitespace()
        .find_map(RawCodeData::from_code)
        // if it is a url from the official deck builder
        .or_else(|| {
            code.split_terminator(&['=', '?']).find_map(|s| {
                urlencoding::decode(s)
                    .as_deref()
                    .ok()
                    .and_then(RawCodeData::from_code)
            })
        })
        .ok_or_else(|| anyhow!("Unable to parse deck code. Code may be invalid."))?;

    let title = code
        .split_once("###")
        .and_then(|(_, s)| s.split_once("# ")) // space added to allow for titles that have #1 in them.
        .filter(|(s, _)| !s.trim().is_empty())
        .map(|(s, _)| s.trim().to_compact_string());

    Ok(raw_data_to_deck(opts, raw_data, title))
}

fn raw_data_to_deck(
    opts: LookupOptions<'_>,
    raw_data: RawCodeData,
    title: Option<CompactString>,
) -> Deck {
    let get_deck_w_code = || -> Result<Deck> {
        let deck = AGENT
            .get("https://us.api.blizzard.com/hearthstone/deck")
            .header("Authorization", format!("Bearer {}", get_access_token()))
            .query("locale", opts.locale.to_compact_string())
            .query("code", &raw_data.deck_code)
            .call()?
            .body_mut()
            .read_json::<Deck>()?;

        anyhow::ensure!(deck.invalid_card_ids.is_empty(), "Deck has invalid IDs");

        Ok(deck)
    };

    let get_deck_w_cards = || -> Result<Deck> {
        let mut req = AGENT
            .get("https://us.api.blizzard.com/hearthstone/deck")
            .header("Authorization", format!("Bearer {}", get_access_token()))
            .query("locale", opts.locale.to_compact_string())
            .query("hero", raw_data.hero.to_compact_string())
            .query(
                "ids",
                raw_data.cards.iter().map(|id| validate_id(*id)).join(","),
            );

        if raw_data.sideboard_cards.is_empty().not() {
            req = req.query(
                "sideboardCards",
                raw_data
                    .sideboard_cards
                    .iter()
                    .map(|(id, sb_id)| {
                        format_compact!("{}:{}", validate_id(*id), validate_id(*sb_id))
                    })
                    .join(","),
            );
        }

        let deck = req.call()?.body_mut().read_json::<Deck>()?;

        anyhow::ensure!(
            deck.invalid_card_ids.iter().all(|&id| id != 0),
            "Deck invalid IDs are 0."
        );

        Ok(deck)
    };

    let get_dummy_deck = || -> Deck {
        Deck {
            title: "Hearthstone Deck".into(),
            deck_code: raw_data.deck_code.clone(),
            format: Format::Standard,
            class: Class::Mage,
            cards: raw_data.cards.iter().map(|&id| Card::dummy(id)).collect(),
            sideboard_cards: raw_data
                .sideboard_cards
                .iter()
                .chunk_by(|(_, sb_card)| sb_card)
                .into_iter()
                .map(|(&sb_card, sb)| Sideboard {
                    sideboard_card: Card::dummy(sb_card),
                    cards_in_sideboard: sb.map(|&(c, _)| Card::dummy(c)).collect(),
                })
                .collect(),
            invalid_card_ids: Vec::new(),
        }
    };

    let mut deck = get_deck_w_code()
        .or_else(|e| {
            tracing::warn!("Encountered error validating code from Blizzard's servers: {e}. Using direct card data instead.");
            get_deck_w_cards()
        })
        .unwrap_or_else(|e| {
            tracing::warn!("Encountered error validating cards from Blizzard's servers: {e}. Using dummy data instead.");
            get_dummy_deck()
        });

    deck.format = opts
        .format
        .as_ref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(raw_data.format);

    deck.title = title.unwrap_or(deck.title);

    // if the deck has invalid card IDs, add dummy cards with backup Data from HearthSim.
    for id in deck.invalid_card_ids.iter() {
        deck.cards.push(Card::dummy(*id));
    }

    specific_card_adjustments(&mut deck);

    deck
}

fn specific_card_adjustments(deck: &mut Deck) {
    // This function contains specific adjustments to specific cards as needed.

    // Treatments for Zilliax Deluxe 3000
    'zilliax_deluxe_3000: {
        const ZILLIAX_DELUXE_3000_ID: usize = 102_983;
        let Some(sb) = deck
            .sideboard_cards
            .iter_mut()
            .find(|sb| sb.sideboard_card.id == ZILLIAX_DELUXE_3000_ID)
        else {
            break 'zilliax_deluxe_3000;
        };

        // removes Cosmetic Modules
        sb.cards_in_sideboard.retain(|c| c.cosmetic.not());

        let (zilliax_cost, zilliax_attack, zilliax_health) =
            sb.cards_in_sideboard
                .iter()
                .fold((0, 0, 0), |(acc_c, acc_a, acc_h), c| {
                    let (a, h) = c.stats();
                    (
                        acc_c + c.cost,
                        acc_a + a.unwrap_or_default(),
                        acc_h + h.unwrap_or_default(),
                    )
                });

        if let Some(Card {
            cost,
            card_type: CardType::Minion { attack, health, .. },
            ..
        }) = deck
            .cards
            .iter_mut()
            .find(|c| c.id == ZILLIAX_DELUXE_3000_ID)
        {
            *cost = zilliax_cost;
            *attack = zilliax_attack;
            *health = zilliax_health;
        }
    }
}

fn format_count(count: usize) -> CompactString {
    if count > 1 {
        format_compact!("{count}x")
    } else {
        CompactString::default()
    }
}

#[cfg(test)]
#[allow(clippy::unreadable_literal)]
mod deck_code_tests {
    use super::*;

    macro_rules! test {
        ($name:ident, $code:literal, $format:expr, $hero:literal, $cards:expr, $sb_cards:expr $(,)?) => {
            #[test]
            fn $name() {
                let expected = RawCodeData {
                    format: $format,
                    hero: $hero,
                    cards: $cards,
                    sideboard_cards: $sb_cards,
                    deck_code: $code.into(),
                };
                assert_eq!(RawCodeData::from_code($code).unwrap(), expected);
            }
        };
    }

    test!(
        deck_normal,
        "AAECAfHhBASYxAXzyAXO8Qb/9wYNh/YE8OgFhY4G/7oGkMsGoOIG4eoGn/EGrPEGvvEGwvEG4/EGqPcGAAA=",
        Format::Standard,
        78065,
        vec![
            90648, 91251, 112846, 113663, 80647, 80647, 95344, 95344, 100101, 100101, 105855,
            105855, 107920, 107920, 110880, 110880, 111969, 111969, 112799, 112799, 112812, 112812,
            112830, 112830, 112834, 112834, 112867, 112867, 113576, 113576,
        ],
        vec![],
    );

    test!(
        deck_with_sideboard,
        "AAECAQcK/cQFrNEFtPgF95cGx6QGk6gG+skG0MoGquoGr/EGCo7UBOypBtW6BqS7BvPKBovcBrDiBtjxBrv0Brz0BgABBs2eBv3EBfSzBsekBvezBsekBtDKBv3EBejeBsekBuntBv3EBQAA",
        Format::Standard,
        7,
        vec![
            90749, 92332, 97332, 101367, 102983, 103443, 107770, 107856, 111914, 112815, 76302,
            76302, 103660, 103660, 105813, 105813, 105892, 105892, 107891, 107891, 110091, 110091,
            110896, 110896, 112856, 112856, 113211, 113211, 113212, 113212,
        ],
        vec![
            (102221, 90749),
            (104948, 102983),
            (104951, 102983),
            (107856, 90749),
            (110440, 102983),
            (112361, 90749)
        ],
    );
}
