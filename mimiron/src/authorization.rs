use crate::AGENT;
use anyhow::Result;
use base64::prelude::*;
use parking_lot::RwLock;
use serde::Deserialize;
use std::{
    ops::Add,
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
static BLIZZARD_CLIENT_AUTH: RwLock<Option<(String, String)>> = RwLock::new(None);

pub fn set_blizzard_client_auth(
    id: String,
    secret: String,
) {
    _ = BLIZZARD_CLIENT_AUTH.write().insert((id, secret));
}

fn internal_get_access_token() -> Result<AccessToken> {
    let (id, secret) = BLIZZARD_CLIENT_AUTH.read().clone().unwrap_or_else(|| {
        panic!(
            "Failed to get {} or {}. Set values with set_blizzard_client_auth",
            super::BLIZZARD_CLIENT_ID,
            super::BLIZZARD_CLIENT_SECRET,
        )
    });

    let creds = BASE64_STANDARD_NO_PAD.encode(format!("{id}:{secret}").as_bytes());

    let access_token = AGENT
        .post("https://oauth.battle.net/token")
        .header("Authorization", format!("Basic {creds}"))
        .query("grant_type", "client_credentials")
        .send_empty()?
        .body_mut()
        .read_json::<AccessToken>()?;

    Ok(access_token)
}

pub fn get_access_token() -> String {
    let current_token = TOKEN.read().clone();
    match current_token {
        Some(at) if Instant::now() < at.expiry => at.token,
        _ => {
            TOKEN
                .write()
                .insert(internal_get_access_token().expect("Failed to get access token"))
                .clone()
                .token
        }
    }
}
