use anyhow::Context as _;
use poise::serenity_prelude as serenity;
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;

mod bg_cmds;
mod card_cmds;
mod deck_cmds;
mod helpers;

pub struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[shuttle_runtime::main]
async fn poise(#[shuttle_runtime::Secrets] secret_store: SecretStore) -> ShuttleSerenity {
    let discord_token =
        secret_store.get("DISCORD_TOKEN").context("'DISCORD_TOKEN' was not found")?;

    std::env::set_var(
        "BLIZZARD_CLIENT_ID",
        secret_store.get("BLIZZARD_CLIENT_ID").context("'BLIZZARD_CLIENT_ID' was not found")?,
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
                card_cmds::keyword(),
                bg_cmds::bg(),
                bg_cmds::battlegrounds(),
                bg_cmds::bgtext(),
                bg_cmds::bgtier(),
                deck_cmds::deck(),
                deck_cmds::addband(),
                deck_cmds::deck_context_menu(),
                deck_cmds::deckcomp(),
                deck_cmds::metadeck(),
                deck_cmds::metasnap(),
                helpers::help(),
            ],
            on_error: |error|
                Box::pin(async move {
                    if let Err(e) = helpers::on_error(error).await {
                        tracing::error!("Error while handling error: {}", e);
                    }
                }),
            post_command: |ctx|
                Box::pin(async move {
                    helpers::on_success(&ctx);
                }),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework|
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        )
        .build();

    let client =
        serenity::ClientBuilder::new(discord_token, serenity::GatewayIntents::non_privileged())
            .framework(framework)
            .await
            .map_err(shuttle_runtime::CustomError::new)?;

    Ok(client.into())
}
