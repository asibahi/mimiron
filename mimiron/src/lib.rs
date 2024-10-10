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
    let mut config = ureq::Config::new();
    config.timeouts.connect = Some(std::time::Duration::from_secs(2));
    config.user_agent = Some("mimiron cli https://github.com/asibahi/mimiron".into());

    config.new_agent()
});

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CardSearchResponse<T> {
    pub cards: Vec<T>,
    pub card_count: usize,
}
