use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};
use serde::Deserialize;

#[allow(unused)]
#[derive(Deserialize)]
struct Authorization {
    access_token: String,
    expires_in: i64,
}

pub fn get_access_token() -> Result<String> {
    // need to replace later with something that allows people to input their own creds
    // dotenvy::dotenv()?;
    // let id = std::env::var("BLIZZARD_CLIENT_ID").context("failed to get BLIZZARD_CLIENT_ID from env")?;
    // let secret = std::env::var("BLIZZARD_CLIENT_SECRET").context("failed to get BLIZZARD_CLIENT_SECRET from env")?;

    let id = dotenvy_macro::dotenv!("BLIZZARD_CLIENT_ID");
    let secret = dotenvy_macro::dotenv!("BLIZZARD_CLIENT_SECRET");

    let creds = general_purpose::STANDARD_NO_PAD.encode(format!("{}:{}", id, secret).as_bytes());

    let access_token = ureq::post("https://oauth.battle.net/token")
        .set("Authorization", &format!("Basic {}", creds))
        .query("grant_type", "client_credentials")
        .call()
        .context("call to get access_token failed")?
        .into_json::<Authorization>()
        .context("parsing authorization json failed")?
        .access_token;
    Ok(access_token)
}
