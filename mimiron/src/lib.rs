use isahc::config::Configurable;
use once_cell::sync::Lazy;

mod authorization;
pub mod bg;
pub mod card;
pub mod card_details;
pub mod deck;
mod deck_image;
mod helpers;
pub mod localization;

pub use authorization::get_access_token;
pub use helpers::card_text_to_markdown;

pub(crate) static CLIENT: Lazy<isahc::HttpClient> = Lazy::new(|| {
    isahc::HttpClient::builder()
        .connect_timeout(std::time::Duration::from_secs(2))
        .default_header(
            "user-agent",
            "mimiron cli https://github.com/asibahi/mimiron",
        )
        .build()
        .unwrap()
});
