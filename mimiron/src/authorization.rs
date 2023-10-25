use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine};
use serde::Deserialize;
use std::{
    ops::Add,
    sync::{OnceLock, RwLock},
};

const ID_KEY: &str = "BLIZZARD_CLIENT_ID";
const SECRET_KEY: &str = "BLIZZARD_CLIENT_SECRET";

#[derive(Deserialize)]
struct Authorization {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize, Clone)]
#[serde(from = "Authorization")]
struct AccessToken {
    token: String,
    expiry: std::time::Instant,
}
impl From<Authorization> for AccessToken {
    fn from(value: Authorization) -> Self {
        Self {
            token: value.access_token,
            expiry: std::time::Instant::now().add(std::time::Duration::from_secs(value.expires_in)),
        }
    }
}

static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
static TOKEN: OnceLock<std::sync::RwLock<AccessToken>> = OnceLock::new();

pub(crate) fn get_agent() -> &'static ureq::Agent {
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(2))
            .user_agent("mimiron cli https://github.com/asibahi/mimiron")
            .build()
    })
}

fn internal_get_access_token() -> Result<AccessToken> {
    // need to replace later with something that allows people to input their own creds
    dotenvy::dotenv().ok();
    let id = std::env::var(ID_KEY).map_err(|e| anyhow!("Failed to get {ID_KEY}: {e}"))?;
    let secret =
        std::env::var(SECRET_KEY).map_err(|e| anyhow!("Failed to get {SECRET_KEY}: {e}"))?;

    let creds = general_purpose::STANDARD_NO_PAD.encode(format!("{id}:{secret}").as_bytes());

    let access_token = get_agent()
        .post("https://oauth.battle.net/token")
        .set("Authorization", &format!("Basic {creds}"))
        .query("grant_type", "client_credentials")
        .call()?
        .into_json::<AccessToken>()?;
    Ok(access_token)
}

pub fn get_access_token() -> String {
    let current = TOKEN.get_or_init(|| {
        let t = internal_get_access_token()
            .map_err(|e| {
                eprintln!("Encountered Error: {e}");
                e
            })
            .expect("Failed to get access token");

        RwLock::new(t)
    });

    let AccessToken { token, expiry } = '_lock_read: { current.read().unwrap().clone() };
    if std::time::Instant::now() > expiry {
        let new_token = internal_get_access_token()
            .map_err(|e| {
                eprintln!("Encountered Error: {e}");
                e
            })
            .expect("Failed to get access token");

        let mut u = TOKEN.get().unwrap().write().unwrap();
        *u = new_token.clone();

        new_token.token
    } else {
        token
    }
}
