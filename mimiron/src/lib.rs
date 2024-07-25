use std::sync::LazyLock;

mod authorization;
pub mod bg;
pub mod card;
pub mod card_details;
pub mod deck;
mod deck_image;
pub mod localization;
pub mod meta;
mod text_utils;

pub use authorization::get_access_token;
pub use text_utils::CardTextDisplay;

pub(crate) static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(2))
        .user_agent("mimiron cli https://github.com/asibahi/mimiron")
        .build()
});

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CardSearchResponse<T> {
    pub cards: Vec<T>,
    pub card_count: usize,
}
