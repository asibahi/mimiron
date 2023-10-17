use anyhow::{Context, Result};

mod authorization;
pub mod bg;
pub mod card;
pub mod card_details;
pub mod deck;
mod deck_image;
mod helpers;

pub struct ApiHandle {
    agent: ureq::Agent,
    pub access_token: String,
    pub locale: String,
}

pub fn get_api_handle() -> Result<ApiHandle> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(2))
        .user_agent("mimiron cli https://github.com/asibahi/mimiron")
        .build();

    let access_token: String =
        authorization::get_access_token(&agent).with_context(|| "failed to get access token.")?;

    Ok(ApiHandle {
        agent,
        access_token,
        locale: "en_us".into(),
    })
}
