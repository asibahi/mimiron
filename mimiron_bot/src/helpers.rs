use std::{cell::LazyCell, collections::HashMap, iter::Iterator};

use crate::{Context, Data, Error};
use itertools::Itertools;
use mimiron::{
    bg::Pool,
    card_details::{Class, Rarity},
    localization::Locale,
};
use poise::serenity_prelude as serenity;

/// Help Menu
#[poise::command(slash_command, hide_in_help)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let footer = "This bot uses the Blizzard API, which mirrors the official card library, \
                  with supplemental data from HearthSim and Firestone. Code is available at \
                  https://github.com/asibahi/mimiron/ . If you have requests or suggestions, \
                  raise a GitHub Issue or ping @mimirons_head in the Mimiron Bot server. The bot \
                  is hosted on the free tier of http://shuttle.rs .";

    // funny new ordering every call.
    let mut categories = HashMap::new();

    for cmd in &ctx.framework().options().commands {
        categories.entry(cmd.category.as_deref()).or_insert(Vec::new()).push(cmd);
    }

    let fields =
        categories.into_iter().filter(|(_, cmds)| !cmds.is_empty()).map(|(category, cmds)| {
            let cmds = cmds
                .into_iter()
                .filter(|cmd| !cmd.hide_in_help)
                // get context menu commands at the bottom.
                .sorted_by_key(|cmd| cmd.slash_action.is_none())
                .map(|cmd| {
                    let name = cmd.context_menu_name.as_deref().unwrap_or(&cmd.name);
                    let prefix = cmd.slash_action.map_or("Context menu: `", |_| "`/");
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

pub trait Emoji {
    fn emoji(&self) -> &'static str;
}
impl Emoji for Class {
    fn emoji(&self) -> &'static str {
        match self {
            Self::DeathKnight => "<:dk:1182031994822086786>",
            Self::DemonHunter => "<:dh:1182032009359528116>",
            Self::Druid => "<:dr:1182032011184066650>",
            Self::Hunter => "<:hu:1182032019052576878>",
            Self::Mage => "<:ma:1182032003177127937>",
            Self::Paladin => "<:pa:1182032015890063403>",
            Self::Priest => "<:pr:1182032001667182732>",
            Self::Rogue => "<:ro:1182031993064665088>",
            Self::Shaman => "<:sh:1182031998802464808>",
            Self::Warlock => "<:wk:1182032014757601340>",
            Self::Warrior => "<:wr:1182032006171861152>",
            Self::Neutral => "",
        }
    }
}
impl Emoji for Rarity {
    fn emoji(&self) -> &'static str {
        match self {
            Self::Legendary => "<:legendary:1182038161099067522>",
            Self::Epic => "<:epic:1182038156841844837>",
            Self::Rare => "<:rare:1182038164781678674>",
            Self::Noncollectible => "<:artifact:1189986811079045282>",
            Self::Common | Self::Free => "<:common:1182038153767419986>",
        }
    }
}
impl Emoji for Pool {
    fn emoji(&self) -> &'static str {
        match self {
            Pool::Solos => ":one: ",
            Pool::Duos => ":two: ",
            Pool::All => ":one: :two: ",
        }
    }
}

pub(crate) async fn on_error(
    error: poise::FrameworkError<'_, Data, Error>,
) -> Result<(), serenity::Error> {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            let command = ctx.command().name.as_str();
            let guild = ctx.guild().map_or("Direct Messages".into(), |g| g.name.clone());

            let invocation = ctx.invocation_string();
            let mut error = error.to_string();
            if rand::random::<u8>() % 5 == 0 && ctx.command().category != Some("Deck".into()) {
                error += "\nOther ways to search can be found in /help.";
            }

            tracing::warn!(command, guild, invocation, error, "Command returned an error.");
            ctx.say(error).await?;
        }
        error => poise::builtins::on_error(error).await?,
    }
    Ok(())
}

pub(crate) fn on_success(ctx: &Context) {
    let command = ctx.command().name.as_str();
    let guild = ctx.guild().map_or("Direct Messages".into(), |g| g.name.clone());

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
        .take(90)
        .map(|c| LazyCell::new(|| inner_card_embed(c)))
        .chunks(3)
        .into_iter()
        .map(Iterator::collect::<Vec<_>>)
        .collect::<Vec<_>>();
    let mut current_page = 0;

    let mut reply = poise::CreateReply::default();
    reply.embeds.extend(embed_chunks[current_page].iter().map(LazyCell::force).cloned());

    if embed_chunks.len() <= 1 {
        ctx.send(reply).await?;
        return Ok(());
    }

    let ctx_id = ctx.id();

    let prev_button =
        serenity::CreateButton::new(format!("{ctx_id}prev")).label("<").disabled(true);

    let pages_indicator = serenity::CreateButton::new("pagination_view")
        .label(format!("{}/{}", current_page + 1, embed_chunks.len()))
        .style(serenity::ButtonStyle::Secondary)
        .disabled(true);

    let next_button = serenity::CreateButton::new(format!("{ctx_id}next")).label(">");

    reply = reply.components(vec![serenity::CreateActionRow::Buttons(vec![
        prev_button.clone(),
        pages_indicator.clone(),
        next_button.clone(),
    ])]);

    let msg = ctx.send(reply).await?;

    // Code copied from poise pagination sample with relevant edits. See comments there for explanation
    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
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
            pages_indicator.clone().label(format!("{}/{}", current_page + 1, embed_chunks.len())),
            next_button.clone().disabled(current_page == embed_chunks.len() - 1),
        ];

        let content = embed_chunks[current_page].iter().map(LazyCell::force).cloned().collect_vec();

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

    last_reply.embeds.extend(embed_chunks[current_page].iter().map(LazyCell::force).cloned());

    msg.edit(ctx, last_reply).await?;

    Ok(())
}

pub(crate) fn get_server_locale(ctx: &Context<'_>) -> Locale {
    match (ctx.guild(), ctx.locale()) {
        (Some(g), _) => g.preferred_locale.parse().unwrap_or_default(),
        (_, Some(l)) => l.parse().unwrap_or_default(),
        _ => Locale::enUS, // surely unreachable?
    }
}
