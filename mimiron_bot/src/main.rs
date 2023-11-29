use anyhow::Context as _;
use poise::serenity_prelude as serenity;
use shuttle_poise::ShuttlePoise;
use shuttle_secrets::SecretStore;

mod bg_cmds;
mod card_cmds;
mod deck_cmds;

pub struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

fn markdown(i: &str) -> String {
    mimiron::card_text_to_markdown(i)
}

#[poise::command(slash_command, hide_in_help)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<(), Error> {
    let configuration = poise::builtins::HelpConfiguration {
        ephemeral: true,
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), configuration).await?;
    Ok(())
}

#[shuttle_runtime::main]
async fn poise(#[shuttle_secrets::Secrets] secret_store: SecretStore) -> ShuttlePoise<Data, Error> {
    // The below code is almost the template from `cargo shuttle init`
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    std::env::set_var(
        "BLIZZARD_CLIENT_ID",
        secret_store
            .get("BLIZZARD_CLIENT_ID")
            .context("'BLIZZARD_CLIENT_ID' was not found")?,
    );
    std::env::set_var(
        "BLIZZARD_CLIENT_SECRET",
        secret_store
            .get("BLIZZARD_CLIENT_SECRET")
            .context("'BLIZZARD_CLIENT_SECRET' was not found")?,
    );

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                card_cmds::card(),
                card_cmds::cardtext(),
                card_cmds::cardreprints(),
                card_cmds::allcards(),
                bg_cmds::battlegrounds(),
                deck_cmds::deck(),
                deck_cmds::addband(),
                help(),
            ],

            ..Default::default()
        })
        .token(discord_token)
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build()
        .await
        .map_err(shuttle_runtime::CustomError::new)?;

    Ok(framework.into())
}
