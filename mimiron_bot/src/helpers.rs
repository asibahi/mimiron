use crate::{Context, Data, Error};
use itertools::Itertools;
use mimiron::{
    bg::Pool,
    card_details::{Class, Rarity},
    localization::Locale,
};
use poise::serenity_prelude as serenity;
use std::{cell::LazyCell, collections::HashMap, ops::Not};

const FOOTER: &str = "This bot uses the Blizzard API, which mirrors the official card library, \
                      with supplemental data from HearthSim and Firestone. Code is available at \
                      https://github.com/asibahi/mimiron/ . If you have requests or suggestions, \
                      raise a GitHub Issue or ping @mimirons_head in the Mimiron Bot server. The bot \
                      is hosted on the free tier of http://shuttle.rs .";

/// Help Menu
#[poise::command(slash_command, install_context = "Guild|User", hide_in_help)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    // ego inflation
    if ctx.guild().is_none() && ctx.framework().options().owners.contains(&ctx.author().id) {
        let reply = poise::CreateReply::default()
            .content(env!("CARGO_PKG_VERSION").to_string())
            .ephemeral(true);

        ctx.send(reply).await?;

        // registeration buttons
        poise::builtins::register_application_commands_buttons(ctx).await?;

        return Ok(());
    }

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
        .filter(|(_, cmds)| cmds.is_empty().not())
        .map(|(category, cmds)| {
            let cmds = cmds
                .into_iter()
                .filter(|cmd| cmd.hide_in_help.not())
                // get context menu commands at the bottom.
                .sorted_by_key(|cmd| cmd.slash_action.is_none())
                .map(|cmd| {
                    format!(
                        "{}{}`: _{}_",
                        cmd.slash_action.map_or("Context menu: `", |_| "`/"),
                        cmd.context_menu_name.as_deref().unwrap_or(&cmd.name),
                        cmd.description.as_deref().unwrap_or_default()
                    )
                })
                .join("\n");

            (category.unwrap_or_default(), cmds, false)
        });

    let embed = serenity::CreateEmbed::new()
        .title("Help")
        .fields(fields)
        .footer(serenity::CreateEmbedFooter::new(format!(
            "{} v:{}",
            FOOTER,
            env!("CARGO_PKG_VERSION"),
        )));

    let reply = poise::CreateReply::default().embed(embed).ephemeral(true);

    ctx.send(reply).await?;

    Ok(())
}

/// News of Hearthstone
#[poise::command(slash_command, install_context = "Guild|User", category = "General")]
pub async fn news(ctx: Context<'_>) -> Result<(), Error> {
    let news = mimiron::news::get_news()?;

    paginated_embeds(ctx, news, |news| {
        serenity::CreateEmbed::new()
            .title(news.title)
            .url(news.default_url)
            .thumbnail(news.thumbnail.url)
            .description(news.summary)
    })
    .await
}

/// Patch Time. Next Tuesday or Thurday 10am Pacific
#[poise::command(slash_command, install_context = "Guild|User", category = "General")]
pub async fn patchtime(ctx: Context<'_>) -> Result<(), Error> {
    use jiff::{
        Zoned,
        civil::{Time, Weekday},
        tz::TimeZone,
    };
    let now = Zoned::now().with_time_zone(TimeZone::get("America/Los_Angeles")?);

    let mut patch = now.with().time(Time::constant(10, 0, 0, 0)).build()?;

    while patch < now || !matches!(patch.weekday(), Weekday::Tuesday | Weekday::Thursday) {
        patch = patch.tomorrow()?;
    }

    let reply = poise::CreateReply::default().content(format!(
        "<t:{0}:F> <t:{0}:R>",
        patch.timestamp().as_second()
    ));

    ctx.send(reply).await?;

    Ok(())
}

pub trait Emoji: Copy {
    fn emoji(self) -> &'static str;
}
impl Emoji for Class {
    fn emoji(self) -> &'static str {
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
        }
    }
}
impl Emoji for Rarity {
    fn emoji(self) -> &'static str {
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
    fn emoji(self) -> &'static str {
        match self {
            Self::Solos => ":one: ",
            Self::Duos => ":two: ",
            Self::All => ":one: :two: ",
        }
    }
}

pub async fn on_error(
    error: poise::FrameworkError<'_, Data, Error>
) -> Result<(), serenity::Error> {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            let command = ctx.command().name.as_str();
            let guild = ctx.guild().map_or_else(
                || format!("User: {}", ctx.author().name),
                |g| g.name.clone(),
            );

            let invocation = ctx.invocation_string();
            let mut error = error.to_string();
            if rand::random::<u8>() % 5 == 0 && ctx.command().category != Some("Deck".into()) {
                error += "\nOther ways to search can be found in /help.";
                if rand::random::<u8>() % 3 == 0 {
                    error += "e.g. /allcards or /cardtext.";
                }
            }

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

pub fn on_success(ctx: &Context<'_>) {
    let command = ctx.command().name.as_str();
    let guild = ctx.guild().map_or_else(
        || format!("User: {}", ctx.author().name),
        |g| g.name.clone(),
    );

    let invocation = ctx.invocation_string();

    tracing::info!(command, guild, invocation, "Command called successfully.");
}

pub async fn terse_embeds<T>(
    ctx: Context<'_>,
    count: usize,
    items: impl Iterator<Item = T> + Send,
    inner_embed: impl Fn(T) -> serenity::CreateEmbed + Send,
) -> Result<(), Error> {
    let items = items.take(count);
    let embeds = items.map(inner_embed);

    let mut reply = poise::CreateReply::default();
    reply.embeds.extend(embeds);

    ctx.send(reply).await?;

    Ok(())
}

pub async fn paginated_embeds<T: Send>(
    ctx: Context<'_>,
    items: impl Iterator<Item = T> + Send,
    inner_embed: impl Fn(T) -> serenity::CreateEmbed + Send + Sync,
) -> Result<(), Error> {
    // pagination elements
    let embed_chunks = items
        .take(90)
        .map(|c| LazyCell::new(|| inner_embed(c)))
        .chunks(3)
        .into_iter()
        .map(Iterator::collect::<Vec<_>>)
        .collect::<Vec<_>>();
    let mut current_page = 0;

    let mut reply = poise::CreateReply::default();
    reply.embeds.extend(
        embed_chunks[current_page]
            .iter()
            .map(LazyCell::force)
            .cloned(),
    );

    if embed_chunks.len() <= 1 {
        ctx.send(reply).await?;
        return Ok(());
    }

    let ctx_id = ctx.id();

    let prev_button = serenity::CreateButton::new(format!("{ctx_id}prev"))
        .label("<")
        .disabled(true);

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
            pages_indicator
                .clone()
                .label(format!("{}/{}", current_page + 1, embed_chunks.len())),
            next_button
                .clone()
                .disabled(current_page == embed_chunks.len() - 1),
        ];

        let content = embed_chunks[current_page]
            .iter()
            .map(LazyCell::force)
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

    last_reply.embeds.extend(
        embed_chunks[current_page]
            .iter()
            .map(LazyCell::force)
            .cloned(),
    );

    msg.edit(ctx, last_reply).await?;

    Ok(())
}

pub fn get_server_locale(ctx: &Context<'_>) -> Locale {
    match (ctx.guild(), ctx.locale()) {
        (Some(g), _) => g.preferred_locale.parse().unwrap_or_default(),
        (_, Some(l)) => l.parse().unwrap_or_default(),
        _ => Locale::enUS, // surely unreachable?
    }
}
