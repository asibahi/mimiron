use crate::{
    card_details::Class,
    deck::{lookup, Deck, Format, LookupOptions},
    localization::Locale,
    AGENT,
};
use anyhow::Result;
use convert_case::{Case, Casing};
use itertools::Itertools;
use serde::Deserialize;

// Meta look up using Firestone's internal data.

// Standard
static STANDARD_DECKS_D_L: &str = "https://static.zerotoheroes.com/api/constructed/stats/decks/standard/legend-diamond/past-3/overview-from-hourly.gz.json";
static STANDARD_DECKS_ALL: &str = "https://static.zerotoheroes.com/api/constructed/stats/decks/standard/all/past-3/overview-from-hourly.gz.json";

// Wild
static WILD_DECKS_D_L: &str = "https://static.zerotoheroes.com/api/constructed/stats/decks/wild/legend-diamond/past-3/overview-from-hourly.gz.json";
static WILD_DECKS_ALL: &str = "https://static.zerotoheroes.com/api/constructed/stats/decks/wild/all/past-3/overview-from-hourly.gz.json";

// Twist
static TWIST_DECKS_D_L: &str = "https://static.zerotoheroes.com/api/constructed/stats/decks/twist/legend-diamond/past-3/overview-from-hourly.gz.json";
static TWIST_DECKS_ALL: &str = "https://static.zerotoheroes.com/api/constructed/stats/decks/twist/all/past-3/overview-from-hourly.gz.json";

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FirestoneStats {
    // last_updated: String, // Do I really care about this?
    deck_stats: Vec<DeckStat>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DeckStat {
    decklist: String,
    // last_update: String, // If I care about it, how can I use it?
    player_class: Class, // Useful for quick filtering
    total_games: u32,
    total_wins: u32,
    winrate: Option<f64>,
    archetype_name: String,
}
impl DeckStat {
    fn get_winrate(&self) -> f64 {
        self.winrate.unwrap_or(f64::from(self.total_wins) / f64::from(self.total_games))
    }
}

#[cached::proc_macro::cached(
    time = 86400, // one day.
    result = true,
)]
fn get_firestone_data(link: &'static str) -> Result<FirestoneStats> {
    let ret = AGENT.get(link).call()?.into_json::<FirestoneStats>()?;
    Ok(ret)
}

pub fn meta_deck(
    class: Option<Class>,
    format: &Format,
    locale: Locale,
) -> Result<impl Iterator<Item = Deck>> {
    let decks =
        get_decks_stats(format, class)?.filter_map(move |ds| get_deck_from_deck_stat(ds, locale));

    Ok(decks)
}

pub fn meta_snap(format: &Format, locale: Locale) -> Result<impl Iterator<Item = Deck>> {
    let decks = get_decks_stats(format, None)?
        .unique_by(|ds| ds.archetype_name.clone())
        .filter_map(move |ds| get_deck_from_deck_stat(ds, locale));

    Ok(decks)
}

fn get_deck_from_deck_stat(ds: DeckStat, locale: Locale) -> Option<Deck> {
    let title = format!(
        "{:.0}% WR {}/{} {}",
        ds.get_winrate() * 100.0,
        ds.total_wins,
        ds.total_games,
        ds.archetype_name.to_case(Case::Title),
    );

    let mut deck = lookup(&LookupOptions::lookup(ds.decklist).with_locale(locale)).ok()?;
    deck.title = title;

    Some(deck)
}

fn get_decks_stats(format: &Format, class: Option<Class>) -> Result<std::vec::IntoIter<DeckStat>> {
    let (d_l, all) = match format {
        Format::Standard => (STANDARD_DECKS_D_L, STANDARD_DECKS_ALL),
        Format::Wild => (WILD_DECKS_D_L, WILD_DECKS_ALL),
        Format::Twist => (TWIST_DECKS_D_L, TWIST_DECKS_ALL),
        _ => anyhow::bail!("Meta decks for this format are not available"),
    };

    let filter_decks = |s: &DeckStat| {
        s.total_games > 100 && (class.is_none() || class.is_some_and(|c| c == s.player_class))
    };

    let first_try = get_firestone_data(d_l)?;
    let mut decks = first_try.deck_stats.into_iter().filter(filter_decks).peekable();

    if decks.peek().is_none() {
        let second_try = get_firestone_data(all)?;
        decks = second_try.deck_stats.into_iter().filter(filter_decks).peekable();
    }

    anyhow::ensure!(decks.peek().is_some(), "No decks found with more than 100 games.");

    let decks = decks.sorted_by(|s1, s2| {
        (s2.total_games.ilog2().min(10))
            .cmp(&s1.total_games.ilog2().min(10))
            .then(s2.get_winrate().total_cmp(&s1.get_winrate()))
    });

    Ok(decks)
}
