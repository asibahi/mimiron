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
use counter::Counter;
use integer_encoding::VarIntReader;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{Display, Write},
    io::Cursor,
    ops::Not,
    str::FromStr,
};

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

    pub fn get_image(&self, opts: ImageOptions) -> Result<image::RgbaImage> {
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

#[derive(Default)]
struct RawCodeData {
    format: Format,
    hero: usize,
    cards: Vec<usize>,
    sideboard_cards: Vec<(usize, usize)>,
    deck_code: String,
}

pub fn lookup(opts: &LookupOptions) -> Result<Deck> {
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
        .find_map(|s| decode_deck_code(s).ok())

        // if it is a url from the official deck builder
        .or_else(|| code
            .split_terminator(&['=', '?'])
            .find_map(|s| urlencoding::decode(s).ok()
                .and_then(|d| decode_deck_code(&d).ok())
            )
        )
        .ok_or(anyhow!("Unable to parse deck code. Code may be invalid."))?;

    let title = code
        .split_once("###")
        .and_then(|(_, s)| s.split_once("# ")) // space added to allow for titles that have #1 in them.
        .map(|(s, _)| s.trim().to_owned());

    Ok(raw_data_to_deck(opts, raw_data, title))
}

fn raw_data_to_deck(opts: &LookupOptions, raw_data: RawCodeData, title: Option<String>) -> Deck {
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
            .query("ids", raw_data.cards.iter().join(","));

        if raw_data.sideboard_cards.is_empty().not() {
            req = req.query(
                "sideboardCards",
                raw_data
                    .sideboard_cards
                    .iter()
                    .map(|(id, sb_id)| format!("{id}:{sb_id}"))
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
                ref mut cost,
                card_type: CardType::Minion { ref mut attack, ref mut health, .. },
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

fn decode_deck_code(code: &str) -> Result<RawCodeData> {
    // Deckstring encoding: https://hearthsim.info/docs/deckstrings/

    const CONFIG: GeneralPurposeConfig =
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent);
    const ENGINE: GeneralPurpose = GeneralPurpose::new(&alphabet::STANDARD, CONFIG);

    let decoded = ENGINE.decode(code)?;
    let mut buffer = Cursor::new(&decoded);

    // while let Ok(n) = buffer.read_varint::<usize>() {
    //     println!("{n}");
    // }

    let mut raw_data = RawCodeData::default();

    // Format is the third number.
    buffer.set_position(2);
    raw_data.format = buffer.read_varint::<u8>()?.try_into().unwrap_or_default();

    // Hero ID is the fifth number.
    buffer.set_position(4);
    raw_data.hero = buffer.read_varint()?;

    // Single copy cards
    let count = buffer.read_varint()?;
    for _ in 0u8..count {
        let id = buffer.read_varint()?;
        let id = validate_id(id);

        raw_data.cards.push(id);
    }

    // Double copy cards
    let count = buffer.read_varint()?;
    for _ in 0u8..count {
        let id = buffer.read_varint()?;
        let id = validate_id(id);

        raw_data.cards.push(id);
        raw_data.cards.push(id); // twice
    }

    // N-copy cards
    let count = buffer.read_varint()?;
    for _ in 0u8..count {
        let id = buffer.read_varint()?;
        let id = validate_id(id);

        let n = buffer.read_varint()?;

        for _ in 0u8..n {
            raw_data.cards.push(id);
        }
    }

    // Sideboard cards. Not sure if they're always available?
    if buffer.read_varint::<u8>().is_ok_and(|i| i == 1) {
        let count = buffer.read_varint()?;
        for _ in 0u8..count {
            let id = buffer.read_varint()?;
            let id = validate_id(id);

            let sb_id = buffer.read_varint()?;
            let sb_id = validate_id(sb_id);

            raw_data.sideboard_cards.push((id, sb_id));
        }
    }

    raw_data.deck_code = ENGINE.encode(decoded); // Hearthstone requires base64 padding

    Ok(raw_data)
}

pub fn add_band(opts: &LookupOptions, band: Vec<String>) -> Result<Deck> {
    // Function WILL need to be updated if new sideboard cards are printed.

    // Constants that might change should ETC be added to core.
    const ETC_NAME: &str = "E.T.C., Band Manager";
    const ETC_ID: usize = 90749;

    let mut raw_data = decode_deck_code(&opts.code)?;

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
            card::lookup(&card::SearchOptions::search_for(name).with_locale(opts.locale))?
                // Undocumented API Found by looking through playhearthstone.com card library
                .map(|c| (c.id, ETC_ID))
                .next()
                .ok_or_else(|| anyhow!("Band found brown M&M's."))
        )
        .collect::<Result<Vec<(_, _)>>>()?;

    raw_data.sideboard_cards.extend(band_ids);

    Ok(raw_data_to_deck(opts, raw_data, None))
}

fn format_count(count: usize) -> String {
    (count > 1).then(|| format!("{count}x")).unwrap_or_default()
}
