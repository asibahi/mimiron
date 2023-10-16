use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine};
use serde::Deserialize;

const ID_KEY: &str = "BLIZZARD_CLIENT_ID";
const SECRET_KEY: &str = "BLIZZARD_CLIENT_SECRET";

#[allow(unused)]
#[derive(Deserialize)]
struct Authorization {
    access_token: String,
    expires_in: i64,
}

pub fn get_access_token(agent: &ureq::Agent) -> Result<String> {
    // need to replace later with something that allows people to input their own creds
    dotenvy::dotenv().ok();
    let id = std::env::var(ID_KEY).map_err(|e| anyhow!("Failed to get {ID_KEY}: {e}"))?;
    let secret =
        std::env::var(SECRET_KEY).map_err(|e| anyhow!("Failed to get {SECRET_KEY}: {e}"))?;

    let creds = general_purpose::STANDARD_NO_PAD.encode(format!("{id}:{secret}").as_bytes());

    let access_token = agent
        .post("https://oauth.battle.net/token")
        .set("Authorization", &format!("Basic {creds}"))
        .query("grant_type", "client_credentials")
        .call()
        .with_context(|| "call to get access_token failed")?
        .into_json::<Authorization>()
        .with_context(|| "parsing authorization json failed")?
        .access_token;
    Ok(access_token)
}
