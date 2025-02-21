use crate::{
    AGENT,
    card_details::Class,
    deck::{Deck, Format, LookupOptions, lookup},
    localization::Locale,
};
use anyhow::{Result, anyhow};
use compact_str::{CompactString, ToCompactString, format_compact};
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
    decklist: CompactString,
    // last_update: String, // If I care about it, how can I use it?
    player_class: Class, // Useful for quick filtering
    total_games: u32,
    total_wins: u32,
    winrate: Option<f64>,
    archetype_name: CompactString,
}
impl DeckStat {
    fn get_winrate(&self) -> f64 {
        self.winrate.unwrap_or_else(|| f64::from(self.total_wins) / f64::from(self.total_games))
    }
}

#[cached::proc_macro::cached(
    time = 86400, // one day.
    result = true,
)]
fn get_firestone_data(link: &'static str) -> Result<FirestoneStats> {
    let mut counter = 5;
    let ret = loop {
        match AGENT.get(link).call() {
            Ok(mut res) => break res.body_mut().read_json::<FirestoneStats>()?,
            Err(ureq::Error::Io(err))
                if counter > 0 && err.kind() == std::io::ErrorKind::ConnectionReset =>
            {   // is this a good idea?
                std::thread::sleep(std::time::Duration::from_millis(500));
                counter -= 1;
                continue;
            }
            err => err?,
        };
    };
    Ok(ret)
}

pub fn meta_deck(
    class: Option<Class>,
    format: Format,
    locale: Locale,
) -> Result<impl Iterator<Item = Deck>> {
    let decks =
        get_decks_stats(format, class)?.filter_map(move |ds| get_deck_from_deck_stat(ds, locale));

    Ok(decks)
}

pub fn meta_snap(format: Format, locale: Locale) -> Result<impl Iterator<Item = Deck>> {
    let decks = get_decks_stats(format, None)?
        .unique_by(|ds| ds.archetype_name.clone())
        .filter_map(move |ds| get_deck_from_deck_stat(ds, locale));

    Ok(decks)
}

pub fn meta_search(search_term: &str, format: Format, locale: Locale) -> Result<Deck> {
    // This function is ridiculous calling parse::<Class>() so often and redundantly.
    let class = search_term
        .split_ascii_whitespace()
        .rev() // Class name is usually last.
        .find_map(|s| s.parse::<Class>().ok());

    get_decks_stats(format, class)?
        .find(|ds| {
            let at = casify_archetype(&ds.archetype_name).to_lowercase();
            at.eq_ignore_ascii_case(search_term.trim())
                // very lame
                || at.split_ascii_whitespace()
                    .any(|s| search_term.to_lowercase().contains(s) && s.parse::<Class>().is_err())
        })
        .and_then(|ds| get_deck_from_deck_stat(ds, locale))
        .ok_or(anyhow!("No deck found with this name in this format."))
}

fn casify_archetype(at: &str) -> CompactString {
    at.split('-')
        .map(|s| if s.eq_ignore_ascii_case("dk") // Death Knight
                || s.eq_ignore_ascii_case("dh")  // Demon Hunter
                || s.eq_ignore_ascii_case("xl")  // X-Large
                || (s.len() <= 3                 // DK Runes
                    && s.chars().all(
                        |c| c.eq_ignore_ascii_case(&'b')
                            || c.eq_ignore_ascii_case(&'f')
                            || c.eq_ignore_ascii_case(&'u')
                    ))
            {
                s.to_compact_string().to_uppercase()
            } else {
                let mut chars = s.chars();
                chars.next().map_or_else(
                    CompactString::default,
                    |first| first.to_uppercase().chain(chars).collect()
                )
            }
        )
        .fold(CompactString::default(), |acc, t|
            if acc.is_empty() {
                t.to_compact_string()
            } else {
                format_compact!("{} {}", acc, t)
            }
        )
}

fn get_deck_from_deck_stat(ds: DeckStat, locale: Locale) -> Option<Deck> {
    let title = format_compact!(
        "{:.0}% WR {}/{} {}",
        ds.get_winrate() * 100.0,
        ds.total_wins,
        ds.total_games,
        casify_archetype(&ds.archetype_name),
    );

    let mut deck = lookup(LookupOptions::lookup(&ds.decklist).with_locale(locale)).ok()?;
    deck.title = title;

    Some(deck)
}

fn get_decks_stats(format: Format, class: Option<Class>) -> Result<impl Iterator<Item = DeckStat>> {
    let (d_l, all, min_count, min_log) = match format {
        Format::Standard => (STANDARD_DECKS_D_L, STANDARD_DECKS_ALL, 100, 10), // 2^10 == 1024
        Format::Wild => (WILD_DECKS_D_L, WILD_DECKS_ALL, 100, 9),              // 2^9  == 512
        Format::Twist => (TWIST_DECKS_D_L, TWIST_DECKS_ALL, 50, 7),            // 2^7  == 128
        _ => anyhow::bail!("Meta decks for this format are not available"),
    };

    let filter_decks =
        |s: &DeckStat| s.total_games > min_count && class.is_none_or(|c| c == s.player_class);

    let first_try = get_firestone_data(d_l)?;
    let mut decks = first_try.deck_stats.into_iter().filter(filter_decks).peekable();

    if decks.peek().is_none() {
        let second_try = get_firestone_data(all)?;
        decks = second_try.deck_stats.into_iter().filter(filter_decks).peekable();
    }

    anyhow::ensure!(decks.peek().is_some(), "No decks found with more than {min_count} games.");

    let decks = decks.sorted_by(|s1, s2|
        (s2.total_games.ilog2().min(min_log))
            .cmp(&s1.total_games.ilog2().min(min_log))
            .then(s2.get_winrate().total_cmp(&s1.get_winrate()))
    );

    Ok(decks)
}
