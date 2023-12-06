use mimiron::card_details::Class;

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

pub fn class_to_emoji(class: Class) -> &'static str {
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
