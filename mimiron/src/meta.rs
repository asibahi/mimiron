use crate::{
    card_details::Class,
    deck::{lookup, Deck, Format, LookupOptions},
    localization::Locale,
    AGENT,
};
use anyhow::Result;
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
    // format: Format, // Don't I already know the format?
    // last_update: String, // If I care about it, how can I use it?
    player_class: Class, // Useful for quick filtering
    total_games: u32,
    total_wins: u32,
    winrate: Option<f64>,
    // archetype_name: String,
}
impl DeckStat {
    fn get_winrate(&self) -> f64 {
        self.winrate.unwrap_or((self.total_wins / self.total_games).into())
    }
}

#[cached::proc_macro::cached(
    time = 604_800, // one week.
    result = true,
)]
fn get_firestone_data(link: &'static str) -> Result<FirestoneStats> {
    let ret = AGENT.get(link).call()?.into_json::<FirestoneStats>()?;
    Ok(ret)
}

#[allow(clippy::needless_pass_by_value)]
pub fn meta_deck(
    class: Option<Class>,
    format: Format,
    locale: Locale,
) -> Result<impl Iterator<Item = Deck>> {
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

    let mut decks = decks
        .sorted_by(|s1, s2| {
            let w1 = s1.get_winrate();
            let w2 = s2.get_winrate();
            w1.total_cmp(&w2).reverse()
        })
        .filter_map(move |ds| {
            let title = format!("Firestone Data: WR:{:.0}%", ds.get_winrate() * 100.0);

            let mut deck = lookup(&LookupOptions::lookup(ds.decklist).with_locale(locale)).ok()?;
            deck.title = title;

            Some(deck)
        })
        .peekable();

    if decks.peek().is_none() {
        anyhow::bail!("No decks found with more than 100 games.");
    }

    Ok(decks)
}
