use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use serde::Deserialize;

#[allow(unused)]
#[derive(Deserialize)]
pub struct Authorization {
    pub access_token: String,
    pub expires_in: i64,
}

pub fn get_access_token(creds: String) -> Result<String, anyhow::Error> {
    let access_token = ureq::post("https://oauth.battle.net/token")
        .set("Authorization", &format!("Basic {}", creds))
        .query("grant_type", "client_credentials")
        .call()?
        .into_json::<Authorization>()?
        .access_token;
    Ok(access_token)
}

pub fn get_creds_from_env() -> Result<String, anyhow::Error> {
    dotenvy::dotenv()?;
    let id = std::env::var("BLIZZARD_CLIENT_ID")?;
    let secret = std::env::var("BLIZZARD_CLIENT_SECRET")?;
    let creds = general_purpose::STANDARD_NO_PAD.encode(format!("{}:{}", id, secret).as_bytes());
    Ok(creds)
}
