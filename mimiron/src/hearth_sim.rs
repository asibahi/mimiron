use crate::{card_details::Rarity, AGENT};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher,
};
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use serde::Deserialize;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

// Hearthstone Json unofficial (from HearthSim)
// Uses https://hearthstonejson.com data for back up if needed.

type HearthSim = HashMap<usize, HearthSimData>;
const REFRESH_RATE: Duration = Duration::from_secs(86400 * 7); // a week

static HEARTH_SIM_IDS: RwLock<Option<(HearthSim, Instant)>> = RwLock::new(None);

fn inner_get_hearth_sim_ids() -> HearthSim {
    AGENT.get("https://api.hearthstonejson.com/v1/latest/enUS/cards.json")
        .call()
        .and_then(|mut res| res.body_mut().read_json::<Vec<HearthSimData>>())
        .map(|v| v.into_iter()
                .filter(|d| d.cost.is_some())
                .map(|d| (d.dbf_id, d))
                .collect::<HashMap<_, _>>()
        )
        .unwrap_or_default()
}

fn get_hearth_sim_ids() -> MappedRwLockReadGuard<'static, HearthSim> {
    let last_update = HEARTH_SIM_IDS.read().as_ref().map(|o| o.1);
    if last_update.is_none_or(|t| t.elapsed() >= REFRESH_RATE) {
        _ = HEARTH_SIM_IDS.write().insert((inner_get_hearth_sim_ids(), Instant::now()));
    }

    RwLockReadGuard::map(HEARTH_SIM_IDS.read(), |c| &c.as_ref().unwrap().0)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HearthSimData {
    dbf_id: usize,
    count_as_copy_of_dbf_id: Option<usize>,
    id: String,
    name: String,
    cost: Option<u8>,
    #[serde(default)]
    rarity: String,
    #[serde(default)]
    collectible: bool,
}

pub fn get_hearth_sim_crop_image(id: usize) -> Option<String> {
    get_hearth_sim_ids()
        .get(&id)
        .map(|c| format!("https://art.hearthstonejson.com/v1/tiles/{}.png", c.id))
}

pub fn get_hearth_sim_details(id: usize) -> Option<(String, u8, Rarity)> {
    get_hearth_sim_ids().get(&id).map(|c| {
        let rarity = match c.rarity.as_str() {
            "LEGENDARY" => Rarity::Legendary,
            "EPIC" => Rarity::Epic,
            "RARE" => Rarity::Rare,
            "COMMON" => Rarity::Common,
            "FREE" => Rarity::Free,
            _ => Rarity::Noncollectible,
        };
        (c.name.clone(), c.cost.unwrap(), rarity)
    })
}

pub fn validate_id(input_id: usize) -> usize {
    get_hearth_sim_ids().get(&input_id).and_then(|c| c.count_as_copy_of_dbf_id).unwrap_or(input_id)
}

pub fn fuzzy_search_hearth_sim(search_term: &str) -> Option<String> {
    // according to the docs doing these here is apparently horribly inefficient.
    // c'est la vie
    let mut matcher = Matcher::new(Config::DEFAULT);
    let results = Pattern::parse(search_term, CaseMatching::Ignore, Normalization::Smart)
        .match_list(
            get_hearth_sim_ids().values().filter(|d| d.collectible).map(|d| d.name.clone()),
            &mut matcher,
        );

    results.first().map(|d| d.0.clone())
}
