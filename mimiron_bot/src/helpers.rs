use crate::{Context, Error};

pub(crate) fn markdown(i: &str) -> String {
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
