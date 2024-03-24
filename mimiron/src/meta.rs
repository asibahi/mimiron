#![allow(unused)]

use crate::{
    card_details::Class,
    deck::{lookup, Deck, Format, LookupOptions},
    localization::{Locale, Localize},
    AGENT,
};
use anyhow::{anyhow, Result};
use itertools::Itertools;
use serde::Deserialize;

// Meta look up using Firestone's internal data.

// Standard
static STANDARD_DECKS_D_L: &str = "https://static.zerotoheroes.com/api/constructed/stats/decks/standard/legend-diamond/last-patch/overview-from-hourly.gz.json";
static STANDARD_DECKS_ALL: &str = "https://static.zerotoheroes.com/api/constructed/stats/decks/standard/all/last-patch/overview-from-hourly.gz.json";

// Wild
static WILD_DECKS_D_L: &str = "https://static.zerotoheroes.com/api/constructed/stats/decks/wild/legend-diamond/last-patch/overview-from-hourly.gz.json";
static WILD_DECKS_ALL: &str = "https://static.zerotoheroes.com/api/constructed/stats/decks/wild/all/last-patch/overview-from-hourly.gz.json";

// Twist
static TWIST_DECKS_D_L: &str = "https://static.zerotoheroes.com/api/constructed/stats/decks/twist/legend-diamond/last-patch/overview-from-hourly.gz.json";
static TWIST_DECKS_ALL: &str = "https://static.zerotoheroes.com/api/constructed/stats/decks/twist/all/last-patch/overview-from-hourly.gz.json";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FirestoneStats {
    // last_updated: String, // Do I really care about this?
    deck_stats: Vec<DeckStat>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeckStat {
    decklist: String,
    // format: Format, // Don't I already know the format?
    // last_update: String, // If I care about it, how can I use it?
    player_class: Class, // Useful for quick filtering
    total_games: usize,
    total_wins: usize,
    winrate: Option<f64>,
    archetype_name: String,
}

#[allow(clippy::needless_pass_by_value)]
pub fn meta_deck(class: Class, format: Format, locale: Locale) -> Result<Deck> {
    let (d_l, all) = match format {
        Format::Standard => (STANDARD_DECKS_D_L, STANDARD_DECKS_ALL),
        Format::Wild => (WILD_DECKS_D_L, WILD_DECKS_ALL),
        Format::Twist => (TWIST_DECKS_D_L, TWIST_DECKS_ALL),
        _ => anyhow::bail!("Format meta decks not available"),
    };

    let first_try = AGENT.get(d_l).call()?.into_json::<FirestoneStats>()?;

    let find_deck = |stats: FirestoneStats| {
        stats
            .deck_stats
            .into_iter()
            .filter(|s| {
                s.player_class == class
                    && s.total_games > 100 // arbitrary?
                    && (s.winrate.is_some_and(|w| w > 0.5) || s.total_wins > (s.total_games / 2))
            })
            .sorted_by(|s1, s2| {
                let w1 = s1.winrate.unwrap_or(50.0);
                let w2 = s2.winrate.unwrap_or(50.0);
                w1.total_cmp(&w2).reverse()
            })
            .next()
    };

    let deck = match find_deck(first_try) {
        Some(d) => d,
        None => {
            let second_try = AGENT.get(all).call()?.into_json::<FirestoneStats>()?;
            find_deck(second_try).ok_or(anyhow!("Could not find a meta deck for this request"))?
        }
    };

    dbg!(deck.winrate);

    lookup(&LookupOptions::lookup(deck.decklist).with_locale(locale))
}
