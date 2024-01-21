use crate::{
    card::{self, Card},
    card_details::Class,
    get_access_token,
    localization::{Locale, Localize},
    CLIENT,
};
use anyhow::{anyhow, Result};
use colored::Colorize;
use counter::Counter;
use futures::{stream, StreamExt, TryStreamExt};
use isahc::AsyncReadResponseExt;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{Display, Write},
};

pub use crate::deck_image::{get as get_image, ImageOptions};

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
    pub format: String,
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
        }
        writeln!(
            buffer,
            "\t{} {}.",
            &self.format.to_uppercase().bold(),
            &self.class.in_locale(locale).to_string().bold()
        )
        .ok();

        let cards = self
            .cards
            .iter()
            .fold(BTreeMap::<_, usize>::new(), |mut map, card| {
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
}

impl LookupOptions {
    #[must_use]
    pub fn lookup(code: String) -> Self {
        Self {
            code,
            locale: Locale::enUS,
        }
    }
    #[must_use]
    pub fn with_locale(self, locale: Locale) -> Self {
        Self { locale, ..self }
    }
}

pub async fn lookup(opts: &LookupOptions) -> Result<Deck> {
    let (title, code) = extract_title_and_code(&opts.code);

    let link = url::Url::parse_with_params(
        "https://us.api.blizzard.com/hearthstone/deck",
        &[
            ("code", code),
            ("locale", &opts.locale.to_string()),
            ("access_token", &get_access_token()),
        ],
    )?;

    let mut deck = CLIENT
        .get_async(link.as_str())
        .await?
        .json::<Deck>()
        .await?;

    // Might need custom error message for wrong status message?
    //     .map_err(|e| match e {
    //         ureq::Error::Status(status, _) => {
    //             anyhow!("Encountered Error: Status {status}. Code may be invalid.")
    //         }
    //         ureq::Error::Transport(e) => anyhow!("Encountered Error: {e}"),
    //     })?

    deck.title = title;

    if let Some(ref invalid_ids) = deck.invalid_card_ids {
        for id in invalid_ids {
            deck.cards.push(card::Card::dummy(*id));
        }
    }

    Ok(deck)
}

pub async fn add_band(opts: &LookupOptions, band: Vec<String>) -> Result<Deck> {
    // Function WILL need to be updated if new sideboard cards are printed.

    // Constants that might change should ETC be added to core.
    const ETC_NAME: &str = "E.T.C., Band Manager";
    const ETC_ID: usize = 90749;

    let deck = lookup(opts).await?;

    if deck.cards.iter().all(|c| c.id != ETC_ID) {
        return Err(anyhow!("{ETC_NAME} does not exist in the deck."));
    }

    if deck.sideboard_cards.is_some() {
        return Err(anyhow!("Deck already has a Sideboard."));
    }

    let card_ids = deck.cards.iter().map(|c| c.id).join(",");

    let band_ids: Vec<String> = stream::iter(band)
        .then(|name| async {
            card::lookup(&card::SearchOptions::search_for(name).with_locale(opts.locale))
                .await?
                // Undocumented API Found by looking through playhearthstone.com card library
                .map(|c| format!("{id}:{ETC_ID}", id = c.id))
                .next()
                .ok_or_else(|| anyhow!("Band found brown M&M's."))
        })
        .try_collect()
        .await?;

    let band_ids = band_ids.join(",");

    let link = url::Url::parse_with_params(
        "https://us.api.blizzard.com/hearthstone/deck",
        &[
            ("locale", &opts.locale.to_string()),
            ("access_token", &get_access_token()),
            ("ids", &card_ids),
            ("sideboardCards", &band_ids),
        ],
    )?;

    let deck = CLIENT
        .get_async(link.as_str())
        .await?
        .json::<Deck>()
        .await?;

    Ok(deck)
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

    let code = code
        .split_ascii_whitespace()
        .find(|s| s.starts_with("AA"))
        .unwrap_or(code);

    (title, code)
}

fn format_count(count: usize) -> String {
    (count > 1).then(|| format!("{count}x")).unwrap_or_default()
}
