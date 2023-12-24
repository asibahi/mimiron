use std::collections::HashMap;

use crate::{Context, Data, Error};
use itertools::Itertools;
use mimiron::card_details::{Class, Rarity};
use once_cell::unsync::Lazy;
use poise::serenity_prelude as serenity;

pub(crate) fn markdown(i: &str) -> String {
    mimiron::card_text_to_markdown(i)
}

#[poise::command(slash_command, hide_in_help)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let footer = "This unofficial bot uses the official Blizzard API, the one used in the official card \
                 library. Code is available at https://github.com/asibahi/mimiron/ . If you have requests \
                 or suggestions, raise a GitHub Issue or ping @asibahi in the Mimiron Bot server. The bot \
                 is hosted on the free tier of http://shuttle.rs .";

    // funny new ordering every call.
    let mut categories = HashMap::new();

    for cmd in &ctx.framework().options().commands {
        categories
            .entry(cmd.category.as_deref())
            .or_insert(Vec::new())
            .push(cmd);
    }

    let fields = categories
        .into_iter()
        .filter(|(_, cmds)| !cmds.is_empty())
        .map(|(category, cmds)| {
            let cmds = cmds
                .into_iter()
                .filter(|cmd| !cmd.hide_in_help)
                // get context menu commands at the bottom.
                .sorted_by_key(|cmd| cmd.slash_action.is_none())
                .map(|cmd| {
                    let name = cmd.context_menu_name.as_deref().unwrap_or(&cmd.name);
                    let prefix = cmd.slash_action.map(|_| "`/").unwrap_or("Context menu: `");
                    format!(
                        "{}{}`: _{}_",
                        prefix,
                        name,
                        cmd.description.as_deref().unwrap_or_default()
                    )
                })
                .join("\n");

            (category.unwrap_or_default(), cmds, false)
        });

    let embed = serenity::CreateEmbed::new()
        .title("Help")
        .fields(fields)
        .footer(serenity::CreateEmbedFooter::new(footer));

    let reply = poise::CreateReply::default().embed(embed).ephemeral(true);

    ctx.send(reply).await?;

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

pub fn rarity_to_emoji(rarity: Rarity) -> &'static str {
    // emojis are in Mimiron Bot server
    match rarity {
        Rarity::Legendary => "<:legendary:1182038161099067522>",
        Rarity::Epic => "<:epic:1182038156841844837>",
        Rarity::Rare => "<:rare:1182038164781678674>",
        _ => "<:common:1182038153767419986>",
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
                "Command returned an error."
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

    tracing::info!(command, guild, invocation, "Command called successfully.");
}

#[allow(unused)] // maybe use for later?
pub(crate) async fn terse_card_print<T>(
    ctx: Context<'_>,
    cards: impl Iterator<Item = T>,
    inner_card_embed: impl Fn(T) -> serenity::CreateEmbed,
) -> Result<(), Error> {
    let cards = cards.take(3);
    let embeds = cards.map(inner_card_embed);

    let mut reply = poise::CreateReply::default();
    reply.embeds.extend(embeds);

    ctx.send(reply).await?;

    Ok(())
}

pub(crate) async fn paginated_card_print<T>(
    ctx: Context<'_>,
    cards: impl Iterator<Item = T>,
    inner_card_embed: impl Fn(T) -> serenity::CreateEmbed,
) -> Result<(), Error> {
    // pagination elements
    let embed_chunks = cards
        .map(|c| Lazy::new(|| inner_card_embed(c)))
        .chunks(3)
        .into_iter()
        .map(|c| c.collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let mut current_page = 0;

    let mut reply = poise::CreateReply::default();
    reply
        .embeds
        .extend(embed_chunks[current_page].iter().map(Lazy::force).cloned());

    if embed_chunks.len() <= 1 {
        ctx.send(reply).await?;
        return Ok(());
    }

    let ctx_id = ctx.id();

    let prev_button = serenity::CreateButton::new(&(format!("{ctx_id}prev")))
        .label("<")
        .disabled(true);

    let pages_indicator = serenity::CreateButton::new("pagination_view")
        .label(format!("{}/{}", current_page + 1, embed_chunks.len()))
        .style(serenity::ButtonStyle::Secondary)
        .disabled(true);

    let next_button = serenity::CreateButton::new(&(format!("{ctx_id}next"))).label(">");

    reply = reply.components(vec![serenity::CreateActionRow::Buttons(vec![
        prev_button.clone(),
        pages_indicator.clone(),
        next_button.clone(),
    ])]);

    let msg = ctx.send(reply).await?;

    // Code copied from poise pagination sample with relevant edits. See comments there for explanation
    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        .timeout(std::time::Duration::from_secs(300)) // 5 minutes
        .await
    {
        current_page = if press.data.custom_id.eq(&(format!("{ctx_id}next"))) {
            (current_page + 1).min(embed_chunks.len() - 1)
        } else {
            current_page.saturating_sub(1)
        };

        let button_row = vec![
            prev_button.clone().disabled(current_page == 0),
            pages_indicator
                .clone()
                .label(format!("{}/{}", current_page + 1, embed_chunks.len())),
            next_button
                .clone()
                .disabled(current_page == embed_chunks.len() - 1),
        ];

        let content = embed_chunks[current_page]
            .iter()
            .map(Lazy::force)
            .cloned()
            .collect_vec();

        press
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .embeds(content)
                        .components(vec![serenity::CreateActionRow::Buttons(button_row)]),
                ),
            )
            .await?;
    }

    let mut last_reply =
        poise::CreateReply::default().components(vec![serenity::CreateActionRow::Buttons(vec![
            prev_button.disabled(true),
            pages_indicator.label(format!("{}/{}", current_page + 1, embed_chunks.len())),
            next_button.disabled(true),
        ])]);

    last_reply
        .embeds
        .extend(embed_chunks[current_page].iter().map(Lazy::force).cloned());

    msg.edit(ctx, last_reply).await?;

    Ok(())
}
