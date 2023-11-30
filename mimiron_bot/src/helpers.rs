use crate::{Context, Data, Error};

pub(crate) fn markdown(i: &str) -> String {
    mimiron::card_text_to_markdown(i)
}

pub(crate) async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    if let Err(e) = match error {
        poise::FrameworkError::Command { error, ctx, .. } => ctx
            .send(
                poise::CreateReply::default()
                    .ephemeral(true)
                    .content(error.to_string()),
            )
            .await
            .map(|_| ()),
        error => poise::builtins::on_error(error).await,
    } {
        tracing::error!("Error while handling error: {}", e);
    }
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
