use crate::{
    card::{self, Card},
    card_details::{validate_id, Class},
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

pub use crate::deck_image::{get as get_image, ImageOptions};

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sideboard {
    pub sideboard_card: Card,
    pub cards_in_sideboard: Vec<Card>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    pub title: Option<String>,
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
}
impl Localize for Deck {
    fn in_locale(&self, locale: Locale) -> impl Display {
        let mut buffer = String::new();

        let code = &self.deck_code;

        if let Some(title) = &self.title {
            writeln!(buffer, "\t{}", title.bold()).ok();
        } else {
            writeln!(
                buffer,
                "\t{} {}.",
                &self.format.to_string().to_uppercase().bold(),
                &self.class.in_locale(locale).to_string().bold()
            )
            .ok();
        }

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
    let (title, code) = extract_title_and_code(&opts.code);

    let raw_data = decode_deck_code(code)?;

    let card_ids = raw_data.cards.iter().join(",");

    let mut deck: ureq::Request = AGENT
        .get("https://us.api.blizzard.com/hearthstone/deck")
        .query("locale", &opts.locale.to_string())
        .query("access_token", &get_access_token())
        .query("ids", &card_ids);

    if !raw_data.sideboard_cards.is_empty(){
        let sb_cards =
            raw_data.sideboard_cards.iter().map(|(id, sb_id)| format!("{id}:{sb_id}")).join(",");
        deck = deck.query("sideboardCards", &sb_cards);
    }

    let mut deck = deck.call()?.into_json::<Deck>()?;

    deck.format = opts.format.as_ref().and_then(|s| s.parse().ok()).unwrap_or(raw_data.format);

    deck.title = title.or_else(|| {
        let hero = AGENT
            .get(&format!("https://us.api.blizzard.com/hearthstone/cards/{}", raw_data.hero))
            .query("locale", &opts.locale.to_string())
            .query("access_token", &get_access_token())
            .query("collectible", "0,1")
            .call()
            .ok()?
            .into_json::<Card>()
            .ok()?
            .name;

        Some(format!("{hero} - {}", deck.format.to_string().to_uppercase()))
    });

    // if the deck has invalid card IDs, add dummy cards.
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

    Ok(deck)
}

#[derive(Default)]
struct RawCodeData {
    format: Format,
    hero: usize,
    cards: Vec<usize>,
    sideboard_cards: Vec<(usize, usize)>,
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
    if let Ok(sb_count) = buffer.read_u8_varint() {
        for _ in 0..sb_count {
            let inner_count = buffer.read_u8_varint()?;

            for _ in 0..inner_count {
                let id = buffer.read_usize_varint()?;
                let id = validate_id(id);

                let sb_id = buffer.read_usize_varint()?;
                let sb_id = validate_id(sb_id);

                raw_data.sideboard_cards.push((id, sb_id));
            }
        }
    }

    Ok(raw_data)
}

pub fn add_band(opts: &LookupOptions, band: Vec<String>) -> Result<Deck> {
    // Function WILL need to be updated if new sideboard cards are printed.

    // Constants that might change should ETC be added to core.
    const ETC_NAME: &str = "E.T.C., Band Manager";
    const ETC_ID: usize = 90749;

    let deck = lookup(opts)?;

    if deck.cards.iter().all(|c| c.id != ETC_ID) {
        return Err(anyhow!("{ETC_NAME} does not exist in the deck."));
    }

    if deck.sideboard_cards.is_some() {
        return Err(anyhow!("Deck already has a Sideboard."));
    }

    let card_ids = deck.cards.iter().map(|c| c.id).join(",");

    let band_ids = band
        .into_iter()
        .map(|name| {
            card::lookup(&card::SearchOptions::search_for(name).with_locale(opts.locale))?
                // Undocumented API Found by looking through playhearthstone.com card library
                .map(|c| format!("{id}:{ETC_ID}", id = c.id))
                .next()
                .ok_or_else(|| anyhow!("Band found brown M&M's."))
        })
        .collect::<Result<Vec<String>>>()?
        .join(",");

    Ok(AGENT
        .get("https://us.api.blizzard.com/hearthstone/deck")
        .query("locale", &opts.locale.to_string())
        .query("access_token", &get_access_token())
        .query("ids", &card_ids)
        .query("sideboardCards", &band_ids)
        .call()?
        .into_json::<Deck>()?)
}

fn extract_title_and_code(code: &str) -> (Option<String>, &str) {
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

    let code = code.split_ascii_whitespace().find(|s| s.starts_with("AA")).unwrap_or(code);

    (title, code)
}

fn format_count(count: usize) -> String {
    (count > 1).then(|| format!("{count}x")).unwrap_or_default()
}

// Deck suggestion look up using d0nkey's site.
// For personal use only unless got permission from d0nkey.

#[allow(clippy::needless_pass_by_value)]
pub fn meta_deck(class: Class, format: Format, locale: Locale) -> Result<Deck> {
    // Standard Demon Hunter Deck.
    // https://www.d0nkey.top/decks?format=2&player_class=DEMONHUNTER

    let class = class.in_en_us().to_string().to_ascii_uppercase().replace(' ', "");
    let fmt_num = match format {
        Format::Standard => "2",
        Format::Wild => "1",
        Format::Twist => "4",
        _ => anyhow::bail!("Format not supported on d0nkey"),
    };

    let req = AGENT
        .get("https://www.d0nkey.top/decks")
        .query("format", fmt_num)
        .query("player_class", &class);

    let fst_try = req.clone().call()?.into_string()?;

    let fst_try =
        fst_try.split_ascii_whitespace().find(|l| l.starts_with("AA") && !l.contains("span"));

    let code = match fst_try {
        Some(code) => code.to_string(),
        None => req
            .query("rank", "all")
            .call()?
            .into_string()?
            .split_ascii_whitespace()
            .find(|l| l.starts_with("AA") && !l.contains("span"))
            .ok_or(anyhow!("No deck found for given class and format."))?
            .to_string(),
    };

    lookup(&(LookupOptions { code, locale, format: None }))
}
