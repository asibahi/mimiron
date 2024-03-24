use once_cell::sync::Lazy;

mod authorization;
pub mod bg;
pub mod card;
pub mod card_details;
pub mod deck;
mod deck_image;
mod helpers;
pub mod localization;
pub mod meta;

pub use authorization::get_access_token;
pub use helpers::CardTextDisplay;

pub(crate) static AGENT: Lazy<ureq::Agent> = Lazy::new(|| {
    ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(2))
        .user_agent("mimiron cli https://github.com/asibahi/mimiron")
        .build()
});
