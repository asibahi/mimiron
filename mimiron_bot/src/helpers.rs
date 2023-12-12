use crate::{Context, Data, Error};
use mimiron::card_details::Class;
use poise::serenity_prelude as serenity;

pub(crate) fn markdown(i: &str) -> String {
    mimiron::card_text_to_markdown(i)
}

#[poise::command(slash_command, hide_in_help)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<(), Error> {
    let configuration = poise::builtins::HelpConfiguration {
        ephemeral: false,
        show_context_menu_commands: true,
        extra_text_at_bottom: "This bot uses the official Blizzard API, the one used in the official card library. \
                               Code is available at https://github.com/asibahi/mimiron/ . If you have requests or \
                               suggestions, raise a GitHub Issue or ping @asibahi in the Mimiron Bot server at \
                               https://discord.gg/Xh6ed56ePV . The bot is hosted on the free tier of http://shuttle.rs .",
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), configuration).await?;
    Ok(())
}

pub fn class_to_emoji(class: Class) -> &'static str {
    // emojis are in Mimiron Bot server
    match class {
        Class::DeathKnight => "<:dk:1182031994822086786>",
        Class::DemonHunter => "<:dh:1182032009359528116>",
        Class::Druid => "<:dr:1182032011184066650>",
        Class::Hunter => "<:hu:1182032019052576878>",
        Class::Mage => "<:ma:1182032003177127937>",
        Class::Paladin => "<:pa:1182032015890063403>",
        Class::Priest => "<:pr:1182032001667182732>",
        Class::Rogue => "<:ro:1182031993064665088>",
        Class::Shaman => "<:sh:1182031998802464808>",
        Class::Warlock => "<:wk:1182032014757601340>",
        Class::Warrior => "<:wr:1182032006171861152>",
        _ => "",
    }
}

pub(crate) async fn on_error(
    error: poise::FrameworkError<'_, Data, Error>,
) -> Result<(), serenity::Error> {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            let command = ctx.command().name.clone();
            let guild = ctx
                .guild()
                .map(|g| g.name.clone())
                .unwrap_or("Direct Messages".into());
            let invocation = ctx.invocation_string();
            let error = error.to_string();
            tracing::warn!(
                command,
                guild,
                invocation,
                error,
                "Command returned an error.\n\tDetails:"
            );
            ctx.say(error).await?;
        }
        error => poise::builtins::on_error(error).await?,
    }
    Ok(())
}

pub(crate) fn on_success(ctx: &Context) {
    let command = ctx.command().name.clone();
    let guild = ctx
        .guild()
        .map(|g| g.name.clone())
        .unwrap_or("Direct Messages".into());

    let invocation = ctx.invocation_string();

    tracing::info!(
        command,
        guild,
        invocation,
        "Command called successfully.\n\tDetails: "
    );
}
