use crate::{
    card::{self, Card},
    card_details::{validate_id, Class, Details},
    get_access_token,
    localization::{Locale, Localize},
    AGENT,
};
use anyhow::{anyhow, Result};
use base64::prelude::*;
use colored::Colorize;
use counter::Counter;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{Display, Write},
    io::Cursor,
    str::FromStr,
};
use varint_rs::VarintReader;

pub use crate::deck_image::ImageOptions;

#[derive(Clone, Default, Deserialize)]
#[serde(from = "String")]
pub enum Format {
    #[default]
    Standard,
    Wild,
    Classic,
    Twist,
    Custom(String),
}
impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Standard => write!(f, "Standard"),
            Format::Wild => write!(f, "Wild"),
            Format::Classic => write!(f, "Classic"),
            Format::Twist => write!(f, "Twist"),
            Format::Custom(fmt) => write!(f, "{fmt}"),
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
            "wild" | "wld" | "w" => Format::Wild,
            "standard" | "std" | "s" => Format::Standard,
            "twist" | "t" => Format::Twist,
            "classic" | "c" => Format::Classic,
            _ => Format::Custom(value),
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
    deck_code: String,
    format: Format,
    hero: Card,
    class: Details,
    cards: Vec<Card>,
    sideboard_cards: Option<Vec<Sideboard>>,
    invalid_card_ids: Option<Vec<usize>>,
}

#[derive(Clone, Deserialize)]
#[serde(from = "DeckData")]
pub struct Deck {
    pub title: String,
    pub deck_code: String,
    pub format: Format,
    pub class: Class,
    pub cards: Vec<Card>,
    pub sideboard_cards: Option<Vec<Sideboard>>,
    invalid_card_ids: Option<Vec<usize>>,
}
impl Deck {
    #[must_use]
    pub fn compare_with(&self, other: &Self) -> DeckDifference {
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

    pub fn get_image(&self, opts: ImageOptions) -> Result<image::DynamicImage> {
        crate::deck_image::get(self, opts)
    }
}
impl From<DeckData> for Deck {
    fn from(value: DeckData) -> Self {
        Deck {
            title: format!("{} - {}", value.hero.name, value.format.to_string().to_uppercase()),
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

        let cards = self.cards.iter().fold(BTreeMap::<_, usize>::new(), |mut map, card| {
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
                    BTreeMap::<_, usize>::new(),
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

    pub deck1_code: String,
    pub deck1_uniques: HashMap<Card, usize>,

    pub deck2_code: String,
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

pub struct LookupOptions {
    code: String,
    locale: Locale,
    format: Option<String>,
}

impl LookupOptions {
    #[must_use]
    pub fn lookup(code: String) -> Self {
        Self { code, locale: Locale::enUS, format: None }
    }
    #[must_use]
    pub fn with_locale(self, locale: Locale) -> Self {
        Self { locale, ..self }
    }
    #[must_use]
    pub fn with_custom_format(self, format: Option<String>) -> Self {
        Self { format, ..self }
    }
}

pub fn lookup(opts: &LookupOptions) -> Result<Deck> {
    let (title, raw_data) = extract_title_and_raw(&opts.code);
    let raw_data = raw_data.ok_or(anyhow!("Unable to parse deck code. Code may be invalid."))?;

    Ok(raw_data_to_deck(opts, raw_data, title))
}

fn raw_data_to_deck(opts: &LookupOptions, raw_data: RawCodeData, title: Option<String>) -> Deck {
    let get_deck_w_code = || -> Result<Deck> {
        let deck = AGENT
            .get("https://us.api.blizzard.com/hearthstone/deck")
            .query("locale", &opts.locale.to_string())
            .query("code", &raw_data.deck_code)
            .query("access_token", &get_access_token())
            .call()?
            .into_json::<Deck>()?;

        if deck.invalid_card_ids.is_some() {
            anyhow::bail!("Deck has invalid IDs.");
        }

        Ok(deck)
    };

    let get_deck_w_cards = || -> Result<Deck> {
        let mut req = AGENT
            .get("https://us.api.blizzard.com/hearthstone/deck")
            .query("locale", &opts.locale.to_string())
            .query("access_token", &get_access_token())
            .query("ids", &raw_data.cards.iter().join(","));

        if !raw_data.sideboard_cards.is_empty() {
            req = req.query(
                "sideboardCards",
                &raw_data
                    .sideboard_cards
                    .iter()
                    .map(|(id, sb_id)| format!("{id}:{sb_id}"))
                    .join(","),
            );
        }

        let deck = req.call()?.into_json::<Deck>()?;

        if deck.invalid_card_ids.as_ref().is_some_and(|ids| ids.iter().any(|&id| id == 0)) {
            anyhow::bail!("Deck invalid IDs are 0.");
        }

        Ok(deck)
    };

    let get_dummy_deck = || -> Deck {
        Deck {
            title: "Hearthstone Deck".into(),
            deck_code: raw_data.deck_code.clone(),
            format: Format::Standard,
            class: Class::Neutral,
            cards: raw_data.cards.iter().map(|&id| card::Card::dummy(id)).collect(),
            sideboard_cards: if raw_data.sideboard_cards.is_empty() {
                None
            } else {
                Some(
                    raw_data
                        .sideboard_cards
                        .iter()
                        .group_by(|(_, sb_card)| sb_card)
                        .into_iter()
                        .map(|(&sb_card, sb)| Sideboard {
                            sideboard_card: card::Card::dummy(sb_card),
                            cards_in_sideboard: sb
                                .into_iter()
                                .map(|&(c, _)| card::Card::dummy(c))
                                .collect(),
                        })
                        .collect(),
                )
            },
            invalid_card_ids: None,
        }
    };

    let mut deck = get_deck_w_code()
        .or_else(|e| {
            eprintln!("Encountered error validating code from Blizzard's servers: {e}.");
            eprintln!("Using direct card data instead.");
            get_deck_w_cards()
        })
        .unwrap_or_else(|e| {
            eprintln!("Encountered error validating cards from Blizzard's servers: {e}.");
            eprintln!("Using dummy data instead.");
            get_dummy_deck()
        });

    deck.format = opts.format.as_ref().and_then(|s| s.parse().ok()).unwrap_or(raw_data.format);

    deck.title = title.unwrap_or(deck.title);

    // if the deck has invalid card IDs, add dummy cards with backup Data from HearthSim.
    if let Some(ref invalid_ids) = deck.invalid_card_ids {
        for id in invalid_ids {
            deck.cards.push(card::Card::dummy(*id));
        }
    }

    // remove cosmetic cards from all sideboards.
    // Currently only has an effect on Zilliax Cosmetic Modules
    for sb in deck.sideboard_cards.iter_mut().flatten() {
        sb.cards_in_sideboard.retain(|c| !c.cosmetic);
    }

    deck
}

#[derive(Default)]
struct RawCodeData {
    format: Format,
    hero: usize,
    cards: Vec<usize>,
    sideboard_cards: Vec<(usize, usize)>,
    deck_code: String,
}

fn decode_deck_code(code: &str) -> Result<RawCodeData> {
    // Deckstring encoding: https://hearthsim.info/docs/deckstrings/

    let decoded = BASE64_STANDARD.decode(code)?;
    let mut buffer = Cursor::new(decoded);
    // while let Ok(id) = buffer.read_usize_varint() {
    //     println!("{}", id);
    // }

    let mut raw_data = RawCodeData::default();

    // Format is the third number.
    buffer.set_position(2);
    raw_data.format = buffer.read_u8_varint()?.try_into().unwrap_or_default();

    // Hero ID is the fifth number.
    buffer.set_position(4);
    raw_data.hero = buffer.read_usize_varint()?;

    // Single copy cards
    let count = buffer.read_u8_varint()?;
    for _ in 0..count {
        let id = buffer.read_usize_varint()?;
        let id = validate_id(id);

        raw_data.cards.push(id);
    }

    // Double copy cards
    let count = buffer.read_u8_varint()?;
    for _ in 0..count {
        let id = buffer.read_usize_varint()?;
        let id = validate_id(id);

        raw_data.cards.push(id);
        raw_data.cards.push(id); // twice
    }

    // N-copy cards
    let count = buffer.read_u8_varint()?;
    for _ in 0..count {
        let id = buffer.read_usize_varint()?;
        let id = validate_id(id);

        let n = buffer.read_u8_varint()?;

        for _ in 0..n {
            raw_data.cards.push(id);
        }
    }

    // Sideboard cards. Not sure if they're always available?
    if buffer.read_u8_varint().is_ok_and(|i| i == 1) {
        let count = buffer.read_u8_varint()?;
        for _ in 0..count {
            let id = buffer.read_usize_varint()?;
            let id = validate_id(id);

            let sb_id = buffer.read_usize_varint()?;
            let sb_id = validate_id(sb_id);

            raw_data.sideboard_cards.push((id, sb_id));
        }
    }

    raw_data.deck_code = code.to_owned();

    Ok(raw_data)
}

pub fn add_band(opts: &LookupOptions, band: Vec<String>) -> Result<Deck> {
    // Function WILL need to be updated if new sideboard cards are printed.

    // Constants that might change should ETC be added to core.
    const ETC_NAME: &str = "E.T.C., Band Manager";
    const ETC_ID: usize = 90749;

    let mut raw_data = decode_deck_code(&opts.code)?;

    if raw_data.cards.iter().all(|&id| id != ETC_ID) {
        anyhow::bail!("{ETC_NAME} does not exist in the deck.");
    }
    if raw_data.sideboard_cards.iter().any(|&(_, id)| id == ETC_ID) {
        anyhow::bail!("Deck already has an {ETC_NAME} Sideboard.");
    }

    let band_ids = band
        .into_iter()
        .map(|name| {
            card::lookup(&card::SearchOptions::search_for(name).with_locale(opts.locale))?
                // Undocumented API Found by looking through playhearthstone.com card library
                .map(|c| (c.id, ETC_ID))
                .next()
                .ok_or_else(|| anyhow!("Band found brown M&M's."))
        })
        .collect::<Result<Vec<(_, _)>>>()?;

    raw_data.sideboard_cards.extend(band_ids);

    Ok(raw_data_to_deck(opts, raw_data, None))
}

fn extract_title_and_raw(code: &str) -> (Option<String>, Option<RawCodeData>) {
    /* For when someone pastes something like this:
     * ### Custom Shaman
     * # etc
     * #
     * AAECAfWfAwjy3QT0oAXmowXipAXFpQX9xAX0yAX00AUL1bIE4LUEssEExc4Exs4Euu0Eyu0EhaoFw9AFxNAFr9EFAAED2aAE/cQFr8MF/cQF0p4G/cQFAAA=
     * #
     * # To use this deck, copy it to your clipboard and create a new deck in Hearthstone
     */

    let title = code
        .split_once("###")
        .and_then(|(_, s)| s.split_once("# ")) // space added to allow for titles that have #1 in them.
        .map(|(s, _)| s.trim().to_owned());

    let raw_data = code.split_ascii_whitespace().find_map(|s| decode_deck_code(s).ok());

    (title, raw_data)
}

fn format_count(count: usize) -> String {
    (count > 1).then(|| format!("{count}x")).unwrap_or_default()
}
