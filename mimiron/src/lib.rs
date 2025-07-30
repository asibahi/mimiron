use std::sync::LazyLock;

mod authorization;
pub mod bg;
pub mod card;
pub mod card_details;
pub mod deck;
mod deck_image;
mod hearth_sim;
pub mod keyword;
pub mod localization;
pub mod meta;
pub mod news;
mod text_utils;

pub use authorization::{get_access_token, set_blizzard_client_auth};
pub use text_utils::CardTextDisplay;

pub const BLIZZARD_CLIENT_ID: &str = "BLIZZARD_CLIENT_ID";
pub const BLIZZARD_CLIENT_SECRET: &str = "BLIZZARD_CLIENT_SECRET";

pub(crate) static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
  ureq::Agent::config_builder()
    .timeout_connect(Some(std::time::Duration::from_secs(2)))
    .user_agent("mimiron cli https://github.com/asibahi/mimiron")
    .build()
    .into()
});

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CardSearchResponse<T> {
  pub cards: Vec<T>,
  pub card_count: usize,
}
