use crate::CLIENT;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine};
use isahc::ReadResponseExt;
use serde::Deserialize;
use std::{
    ops::Add,
    sync::RwLock,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
struct Authorization {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize, Clone)]
#[serde(from = "Authorization")]
struct AccessToken {
    token: String,
    expiry: Instant,
}
impl From<Authorization> for AccessToken {
    fn from(value: Authorization) -> Self {
        Self {
            token: value.access_token,
            expiry: Instant::now().add(Duration::from_secs(value.expires_in)),
        }
    }
}

static TOKEN: RwLock<Option<AccessToken>> = RwLock::new(None);

const ID_KEY: &str = "BLIZZARD_CLIENT_ID";
const SECRET_KEY: &str = "BLIZZARD_CLIENT_SECRET";

fn internal_get_access_token() -> Result<AccessToken> {
    // need to replace later with something that allows people to input their own creds
    dotenvy::dotenv().ok();
    let id = std::env::var(ID_KEY).map_err(|e| anyhow!("Failed to get {ID_KEY}: {e}"))?;
    let secret =
        std::env::var(SECRET_KEY).map_err(|e| anyhow!("Failed to get {SECRET_KEY}: {e}"))?;

    let creds = general_purpose::STANDARD_NO_PAD.encode(format!("{id}:{secret}").as_bytes());

    let link = url::Url::parse_with_params(
        "https://oauth.battle.net/token",
        &[("grant_type", "client_credentials")],
    )?;

    let access_token = CLIENT
        .send(
            isahc::Request::post(link.as_str())
                .header("Authorization", &format!("Basic {creds}"))
                .body(())?,
        )?
        .json::<AccessToken>()?;

    Ok(access_token)
}

pub fn get_access_token() -> String {
    let current_token = TOKEN.read().unwrap().clone();
    match current_token {
        Some(at) if Instant::now() < at.expiry => at.token,
        _ => {
            TOKEN
                .write()
                .unwrap()
                .insert(internal_get_access_token().expect("Failed to get access token"))
                .clone()
                .token
        }
    }
}
