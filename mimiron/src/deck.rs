use crate::{
    card::{self, Card},
    card_details::Class,
    get_access_token, get_agent,
    helpers::Thusable,
};
use anyhow::{anyhow, Result};
use colored::Colorize;
use counter::Counter;
use itertools::Itertools;
use serde::Deserialize;
use std::collections::HashMap;
use std::{collections::BTreeMap, fmt::Display};

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
impl Display for Deck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = &self.deck_code;
        let class = &self.class.to_string().bold();
        let format = &self.format.to_uppercase().bold();
        writeln!(f, "{format:>10} {class} deck.")?;

        let cards = self
            .cards
            .iter()
            .fold(BTreeMap::<_, usize>::new(), |mut map, card| {
                *map.entry(card).or_default() += 1;
                map
            });

        for (card, count) in cards {
            let count = (count > 1).thus_or_default(format!("{count}x"));
            writeln!(f, "{count:>4} {card}")?;
        }

        if let Some(sideboards) = &self.sideboard_cards {
            for sideboard in sideboards {
                writeln!(f, "Sideboard of {}:", sideboard.sideboard_card.name)?;

                let cards = sideboard.cards_in_sideboard.iter().fold(
                    BTreeMap::<_, usize>::new(),
                    |mut map, card| {
                        *map.entry(card).or_default() += 1;
                        map
                    },
                );

                for (card, count) in cards {
                    let count = (count > 1).thus_or_default(format!("{count}x"));
                    writeln!(f, "{count:>4} {card}")?;
                }
            }
        }

        write!(f, "{code}")
    }
}

pub struct DeckDifference {
    pub shared_cards: HashMap<Card, usize>,

    pub deck1_code: String,
    pub deck1_uniques: HashMap<Card, usize>,

    pub deck2_code: String,
    pub deck2_uniques: HashMap<Card, usize>,
}
impl Display for DeckDifference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (card, count) in &self.shared_cards.iter().collect::<BTreeMap<_, _>>() {
            let count = (**count > 1).thus_or_default(format!("{count}x"));
            writeln!(f, "{count:>4} {card}")?;
        }

        writeln!(f, "\n{}", self.deck1_code)?;
        for (card, count) in &self.deck1_uniques.iter().collect::<BTreeMap<_, _>>() {
            let count = (**count > 1).thus_or_default(format!("{count}x"));
            writeln!(f, "{}{count:>3} {card}", "+".green())?;
        }

        writeln!(f, "\n{}", self.deck2_code)?;
        for (card, count) in &self.deck2_uniques.iter().collect::<BTreeMap<_, _>>() {
            let count = (**count > 1).thus_or_default(format!("{count}x"));
            writeln!(f, "{}{count:>3} {card}", "-".red())?;
        }
        Ok(())
    }
}

pub fn lookup(code: &str) -> Result<Deck> {
    let mut deck = get_agent()
        .get("https://us.api.blizzard.com/hearthstone/deck")
        .query("locale", "en-US")
        .query("code", code)
        .query("access_token", get_access_token())
        .call()?
        .into_json::<Deck>()?;

    // ugly hack for double class decks. Doesn't work if card id's don't exist in API.
    // e.g. Works for Duels double class decks.   Doesn't work with Core Brann when Brann is not in Core.
    // Current impl is only one extra API call _but_ doesn't work on potential future triple class decks.
    // Doesn't change the `class` field in the Deck.
    if let Some(ref invalid_ids) = deck.invalid_card_ids {
        eprint!("Code may contain invalid ID's. Double checking ...\r");

        let card_ids = invalid_ids.iter().join(",");

        let response = get_agent()
            .get("https://us.api.blizzard.com/hearthstone/deck")
            .query("locale", "en-US")
            .query("access_token", get_access_token())
            .query("ids", &card_ids)
            .call();

        if let Ok(response) = response {
            if let Ok(mut other_deck) = response.into_json::<Deck>() {
                deck.cards.append(&mut other_deck.cards);
            }
        }

        eprint!("                                                   \r");
    }

    Ok(deck)
}

pub fn add_band(deck: &mut Deck, band: Vec<String>) -> Result<()> {
    // Function WILL need to be updated if new sideboard cards are printed.

    // Constants that might change should ETC be added to core.
    const ETC_NAME: &str = "E.T.C., Band Manager";
    const ETC_ID: usize = 90749;

    if !deck.cards.iter().any(|c| c.id == ETC_ID) {
        return Err(anyhow!("{ETC_NAME} does not exist in the deck."));
    }

    if deck.sideboard_cards.is_some() {
        return Err(anyhow!("Deck already has a Sideboard."));
    }

    let card_ids = deck.cards.iter().map(|c| c.id).join(",");

    let band_ids = band
        .into_iter()
        .map(|name| {
            card::lookup(&card::SearchOptions::search_for(name))?
                // Undocumented API Found by looking through playhearthstone.com card library
                .map(|c| format!("{id}:{ETC_ID}", id = c.id))
                .next()
                .ok_or_else(|| anyhow!("Band found brown M&M's."))
        })
        .collect::<Result<Vec<String>>>()?
        .join(",");

    *deck = get_agent()
        .get("https://us.api.blizzard.com/hearthstone/deck")
        .query("locale", "en-US")
        .query("access_token", get_access_token())
        .query("ids", &card_ids)
        .query("sideboardCards", &band_ids)
        .call()?
        .into_json::<Deck>()?;

    Ok(())
}
