use crate::{
    card::{self, Card},
    card_details::{CardType, Class, Details},
    get_access_token,
    hearth_sim::validate_id,
    localization::{Locale, Localize},
    AGENT,
};
use anyhow::{anyhow, Result};
use base64::{
    alphabet,
    engine::{DecodePaddingMode, Engine as _, GeneralPurpose, GeneralPurposeConfig},
};
use colored::Colorize;
use compact_str::{format_compact, CompactString, ToCompactString};
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{Display, Write},
    ops::Not,
    str::FromStr,
};

pub use crate::deck_image::ImageOptions;

#[derive(Clone, Default, Deserialize)]
#[serde(from = "String")]
pub enum Format { #[default] Standard, Wild, Classic, Twist, Custom(CompactString) }

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    sideboard_cards: Option<Vec<Sideboard>>,
    invalid_card_ids: Option<Vec<usize>>,
}

#[derive(Clone, Deserialize)]
#[serde(from = "DeckData")]
pub struct Deck {
    pub title: CompactString,
    pub deck_code: CompactString,
    pub format: Format,
    pub class: Class,
    pub cards: Vec<Card>,
    pub sideboard_cards: Option<Vec<Sideboard>>,
    invalid_card_ids: Option<Vec<usize>>,
}
impl Deck {
    #[must_use]
    pub fn compare_with(&self, other: &Self) -> DeckDifference {
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
    pub fn get_image(&self, opts: ImageOptions) -> image::RgbaImage {
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
            class: value.class.id.into(),
            cards: value.cards,
            sideboard_cards: value.sideboard_cards,
            invalid_card_ids: value.invalid_card_ids,
        }
    }
}
impl Localize for Deck {
    fn in_locale(&self, locale: Locale) -> impl Display {
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

        if let Some(sideboards) = &self.sideboard_cards {
            for sideboard in sideboards {
                writeln!(buffer, "Sideboard: {}", sideboard.sideboard_card.name).ok();

                let cards = sideboard.cards_in_sideboard.iter().fold(
                    BTreeMap::new(),
                    |mut map, card| {
                        *map.entry(card).or_default() += 1;
                        map
                    },
                );

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
    fn in_locale(&self, locale: Locale) -> impl Display {
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
        Self { code, locale: Locale::enUS, format: None }
    }
    #[must_use]
    pub const fn with_locale(self, locale: Locale) -> Self {
        Self { locale, ..self }
    }
    #[must_use]
    pub const fn with_custom_format(self, format: Option<&'s str>) -> Self {
        Self { format, ..self }
    }
}

struct RawCodeData {
    format: Format,
    hero: usize,
    cards: Vec<usize>,
    sideboard_cards: Vec<(usize, usize)>,
    deck_code: CompactString,
}

const CONFIG: GeneralPurposeConfig =
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent);
const ENGINE: GeneralPurpose = GeneralPurpose::new(&alphabet::STANDARD, CONFIG);

impl RawCodeData {
    fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        // Deckstring encoding: https://hearthsim.info/docs/deckstrings/

        use nom::{
            branch::alt,
            bytes::complete::{take, take_till},
            combinator::{map, map_opt, recognize, success, verify},
            multi::{count, length_count},
            number::complete::u8,
            sequence::{pair, preceded, tuple},
            IResult,
        };

        #[allow(clippy::cast_possible_truncation)]
        fn parse_varint(input: &[u8]) -> IResult<&[u8], usize> {
            let is_last = |b| b & 0x80 == 0;
            let is_in_bounds = |p: &[u8]| p.len() < 9;

            map_opt(
                recognize(pair(verify(take_till(is_last), is_in_bounds), take(1u8))),
                |p: &[u8]| p.iter().enumerate().try_fold(0, |acc, (idx, byte)|
                    ((*byte as usize) & 0x7F).checked_shl(idx as u32 * 7).map(|n| acc | n)
                ),
            )(input)
        }

        #[cfg(debug_assertions)]
        {
            let raw_code = nom::combinator::iterator(input, parse_varint).fuse().join(", ");
            tracing::info!(raw_code);
        }

        // Format is the third number.
        preceded(verify(count(u8, 2), |r| r == vec![0, 1]), map(tuple((
            // format
            map(u8, |f| f.try_into().unwrap_or_default()),

            // hero
            preceded(u8, parse_varint),

            // cards
            map(tuple((
                // single cards
                length_count(u8, parse_varint),

                // double cards
                length_count(u8, map(parse_varint, |id| [id; 2])),

                // n-count cards
                length_count(u8, map(pair(parse_varint, u8), |(id, n)| [id].repeat(n as usize))),
            )),
            |(v1, v2, vn)| v1.into_iter()
                .chain(v2.into_iter().flatten())
                .chain(vn.into_iter().flatten())
                .collect()
            ),

            // sideboard
            alt((
                preceded(
                    verify(u8, |i| *i == 1),
                    length_count(u8, pair(parse_varint, parse_varint)),
                ),
                success(Vec::new()),
            )),
        )),
        |(format, hero, cards, sideboard_cards)| Self {
            format,
            hero,
            cards,
            sideboard_cards,
            deck_code: ENGINE.encode(input).into(), // Hearthstone requires base64 padding
        }))(input)
    }

    fn from_code(code: &str) -> Option<Self> {
        let decoded = ENGINE.decode(code).ok()?;

        Self::parse(&decoded).map(|(_, rd)| rd).ok()
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
        .or_else(|| code.split_terminator(&['=', '?']).find_map(|s|
            urlencoding::decode(s).as_deref().ok().and_then(RawCodeData::from_code)
        ))
        .ok_or_else(|| anyhow!("Unable to parse deck code. Code may be invalid."))?;

    let title = code
        .split_once("###")
        .and_then(|(_, s)| s.split_once("# ")) // space added to allow for titles that have #1 in them.
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
            .query("locale", opts.locale.to_string())
            .query("code", &raw_data.deck_code)
            .header("Authorization", format!("Bearer {}", get_access_token()))
            .call()?
            .body_mut()
            .read_json::<Deck>()?;

        anyhow::ensure!(deck.invalid_card_ids.is_none(), "Deck has invalid IDs");

        Ok(deck)
    };

    let get_deck_w_cards = || -> Result<Deck> {
        let mut req = AGENT
            .get("https://us.api.blizzard.com/hearthstone/deck")
            .query("locale", opts.locale.to_string())
            .header("Authorization", format!("Bearer {}", get_access_token()))
            .query("hero", raw_data.hero.to_string())
            .query("ids", raw_data.cards.iter().map(|id| validate_id(*id)).join(","));

        if raw_data.sideboard_cards.is_empty().not() {
            req = req.query(
                "sideboardCards",
                raw_data
                    .sideboard_cards
                    .iter()
                    .map(|(id, sb_id)| format_compact!("{}:{}", validate_id(*id), validate_id(*sb_id)))
                    .join(","),
            );
        }

        let deck = req.call()?.body_mut().read_json::<Deck>()?;

        anyhow::ensure!(
            deck.invalid_card_ids.as_ref().is_none_or(|ids| ids.iter().all(|&id| id != 0)),
            "Deck invalid IDs are 0."
        );

        Ok(deck)
    };

    let get_dummy_deck = || -> Deck {
        Deck {
            title: "Hearthstone Deck".into(),
            deck_code: raw_data.deck_code.clone(),
            format: Format::Standard,
            class: Class::Neutral,
            cards: raw_data.cards.iter().map(|&id| card::Card::dummy(id)).collect(),
            sideboard_cards: raw_data
                .sideboard_cards
                .iter()
                .chunk_by(|(_, sb_card)| sb_card)
                .into_iter()
                .map(|(&sb_card, sb)| Sideboard {
                    sideboard_card: card::Card::dummy(sb_card),
                    cards_in_sideboard: sb.map(|&(c, _)| card::Card::dummy(c)).collect(),
                })
                .map(Some)
                .collect::<Option<Vec<_>>>()
                .filter(|v| v.is_empty().not()),
            invalid_card_ids: None,
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

    deck.format = opts.format.as_ref().and_then(|s| s.parse().ok()).unwrap_or(raw_data.format);

    deck.title = title.unwrap_or(deck.title);

    // if the deck has invalid card IDs, add dummy cards with backup Data from HearthSim.
    for id in deck.invalid_card_ids.iter().flatten() {
        deck.cards.push(card::Card::dummy(*id));
    }

    specific_card_adjustments(&mut deck);

    deck
}

fn specific_card_adjustments(deck: &mut Deck) {
    // This function contains specific adjustments to specific cards as needed.

    // Treatments for Zilliax Deluxe 3000
    const ZILLIAX_DELUXE_3000_ID: usize = 102_983;
    '_zilliax_deluxe_3000: for sb in deck.sideboard_cards.iter_mut().flatten() {
        // removes cosmetic cards from all sideboards.
        // Currently only has an effect on Zilliax Cosmetic Modules
        sb.cards_in_sideboard.retain(|c| c.cosmetic.not());

        if sb.sideboard_card.id == ZILLIAX_DELUXE_3000_ID {
            let (zilliax_cost, zilliax_attack, zilliax_health) =
                sb.cards_in_sideboard.iter().fold((0, 0, 0), |(acc_c, acc_a, acc_h), c| {
                    let (a, h) = c.stats();
                    (acc_c + c.cost, acc_a + a.unwrap_or_default(), acc_h + h.unwrap_or_default())
                });

            if let Some(Card {
                cost,
                card_type: CardType::Minion { attack, health, .. },
                ..
            }) = deck.cards.iter_mut().find(|c| c.id == ZILLIAX_DELUXE_3000_ID)
            {
                *cost = zilliax_cost;
                *attack = zilliax_attack;
                *health = zilliax_health;
            }
        }
    }
}

pub fn add_band(opts: LookupOptions<'_>, band: Vec<String>) -> Result<Deck> {
    // Function WILL need to be updated if new sideboard cards are printed.

    // Constants that might change should ETC be added to core.
    const ETC_NAME: &str = "E.T.C., Band Manager";
    const ETC_ID: usize = 90749;

    let Some(mut raw_data) = RawCodeData::from_code(opts.code) else {
        anyhow::bail!("Failed to parse code")
    };

    anyhow::ensure!(
        raw_data.cards.iter().any(|&id| id == ETC_ID),
        "{ETC_NAME} does not exist in the deck."
    );

    anyhow::ensure!(
        raw_data.sideboard_cards.iter().all(|&(_, id)| id != ETC_ID),
        "Deck already has an {ETC_NAME} Sideboard."
    );

    let band_ids = band
        .into_iter()
        .map(|name|
            card::lookup(card::SearchOptions::search_for(&name).with_locale(opts.locale))?
                // Undocumented API Found by looking through playhearthstone.com card library
                .map(|c| (c.id, ETC_ID))
                .next()
                .ok_or_else(|| anyhow!("Band found brown M&M's."))
        )
        .collect::<Result<Vec<(_, _)>>>()?;

    raw_data.sideboard_cards.extend(band_ids);

    Ok(raw_data_to_deck(opts, raw_data, None))
}

fn format_count(count: usize) -> CompactString {
    (count > 1).then(|| format_compact!("{count}x")).unwrap_or_default()
}
