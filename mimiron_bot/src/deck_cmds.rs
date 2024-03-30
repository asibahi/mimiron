use crate::{
    helpers::{get_server_locale, Emoji},
    Context, Error,
};
use itertools::Itertools;
use mimiron::{
    card,
    deck::{self, Deck, LookupOptions},
    localization::Localize,
    meta,
};
use once_cell::sync::Lazy;
use poise::serenity_prelude as serenity;
use rand::random;
use std::{collections::HashMap, io::Cursor};

/// Get deck cards from code
#[poise::command(slash_command, category = "Deck")]
pub async fn deck(
    ctx: Context<'_>,
    #[description = "deck code"] code: String,
    #[description = "mode"] format: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    deck_inner(ctx, code, format).await
}

/// Get deck cards from by right-clicking a message with a deck code.
#[poise::command(context_menu_command = "Get Deck", category = "Deck")]
pub async fn deck_context_menu(
    ctx: Context<'_>,
    #[description = "deck code"] msg: serenity::Message,
) -> Result<(), Error> {
    ctx.defer().await?;

    deck_inner(ctx, msg.content, None).await
}

pub async fn deck_inner(
    ctx: Context<'_>,
    code: String,
    format: Option<String>,
) -> Result<(), Error> {
    let locale = get_server_locale(&ctx);

    let opts = LookupOptions::lookup(code).with_locale(locale).with_custom_format(format);

    let deck = deck::lookup(&opts)?;

    send_deck_reply(ctx, deck).await
}

/// Add a band to a deck with ETC but without a band.
#[poise::command(slash_command, category = "Deck")]
pub async fn addband(
    ctx: Context<'_>,
    #[description = "deck code"] code: String,
    #[description = "band member"] member1: String,
    #[description = "band member"] member2: String,
    #[description = "band member"] member3: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = LookupOptions::lookup(code).with_locale(locale);

    let deck = deck::add_band(&opts, vec![member1, member2, member3])?;

    send_deck_reply(ctx, deck).await
}

/// Compare two decks. Provide both codes.
#[poise::command(slash_command, category = "Deck")]
pub async fn deckcomp(
    ctx: Context<'_>,
    #[description = "deck 1 code"] code1: String,
    #[description = "deck 2 code"] code2: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    // Needs more specific localized strings
    let locale = get_server_locale(&ctx);

    let deck1 = deck::lookup(&LookupOptions::lookup(code1).with_locale(locale))?;
    let deck2 = deck::lookup(&LookupOptions::lookup(code2).with_locale(locale))?;
    let deckcomp = deck1.compare_with(&deck2);

    let sort_and_set = |map: HashMap<card::Card, usize>| {
        map.into_iter()
            .sorted()
            .map(|(card, count)| {
                let square = card.rarity.emoji();
                let count = (count > 1).then(|| format!("_{count}x_ ")).unwrap_or_default();

                format!("{} {}{}", square, count, card.name)
            })
            .join("\n")
    };

    let uniques_1 = sort_and_set(deckcomp.deck1_uniques);
    let uniques_2 = sort_and_set(deckcomp.deck2_uniques);
    let shared = sort_and_set(deckcomp.shared_cards);

    let fields = vec![
        ("Code 1", deck1.deck_code, false),
        ("Code 2", deck2.deck_code, false),
        ("Deck 1", uniques_1, true),
        ("Deck 2", uniques_2, true),
        ("Shared", shared, true),
    ];

    let embed = serenity::CreateEmbed::default()
        .title(format!("{} Deck Comparison", deck1.class.in_locale(locale)))
        .color(deck1.class.color())
        .fields(fields);

    let reply = poise::CreateReply::default().embed(embed);

    ctx.send(reply).await?;

    Ok(())
}

async fn send_deck_reply(ctx: Context<'_>, deck: Deck) -> Result<(), Error> {
    ctx.send(create_deck_reply(deck)?).await?;

    Ok(())
}

fn create_deck_reply(deck: Deck) -> Result<poise::CreateReply, Error> {
    let attachment_name = format!(
        "{}.png",
        deck.deck_code.chars().filter(|c| c.is_alphanumeric()).collect::<String>()
    );

    let attachment = {
        let img = deck.get_image(deck::ImageOptions::Adaptable)?;

        let mut image_data = Cursor::new(Vec::<u8>::new());
        img.write_to(&mut image_data, image::ImageFormat::Png)?;

        serenity::CreateAttachment::bytes(image_data.into_inner(), attachment_name.as_str())
    };

    let mut embed = serenity::CreateEmbed::new()
        .title(deck.title)
        .url(format!(
            "https://hearthstone.blizzard.com/deckbuilder?deckcode={}",
            urlencoding::encode(&deck.deck_code)
        ))
        .description(&deck.deck_code)
        .color(deck.class.color())
        .attachment(attachment_name);

    if random::<u8>() % 10 == 0 {
        embed =
            embed.footer(serenity::CreateEmbedFooter::new("See other useful commands with /help."));
    }

    let reply = poise::CreateReply::default().attachment(attachment).embed(embed);

    Ok(reply)
}

/// Get a "meta" deck from Firestone's data.
#[poise::command(slash_command, category = "Deck")]
pub async fn metadeck(
    ctx: Context<'_>,
    #[description = "Class"] class: Option<String>,
    #[description = "Format"] format: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let class = class.and_then(|s| s.parse().ok());
    let format = parse_format(ctx, format).await;

    let deck = meta::meta_deck(class, &format, locale)?
        .take(5)
        .find_or_first(|_| random::<u8>() % 5 == 0)
        .unwrap();

    send_deck_reply(ctx, deck).await
}

/// Get a meta snapshot from Firestone's data.
#[poise::command(slash_command, category = "Deck")]
pub async fn metasnap(
    ctx: Context<'_>,
    #[description = "Format"] format: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let ctx_id = ctx.id();

    let locale = get_server_locale(&ctx);
    let format = parse_format(ctx, format).await;
    let decks = meta::meta_snap(&format, locale)?.enumerate().take(10).collect::<Vec<_>>();

    let embed = serenity::CreateEmbed::new()
        .title(format!("{} Meta Snapshot (from Firestone)", format.to_string().to_uppercase()))
        .url("https://go.overwolf.com/firestone-app/")
        .description(decks.iter().map(|(i, d)| format!("{}. {}", i + 1, d.title)).join("\n"))
        .color(decks[0].1.class.color())
        .footer(serenity::CreateEmbedFooter::new(
            "Best performing deck of each archetype. \
            Data is from the past 3 days. \
            Diamond to Legend (usually).",
        ));

    let select_menu = serenity::CreateSelectMenu::new(
        format!("{ctx_id}_select_menu"),
        serenity::CreateSelectMenuKind::String {
            options: decks
                .iter()
                .map(|(i, d)| serenity::CreateSelectMenuOption::new(&d.title, i.to_string()))
                .collect::<Vec<_>>(),
        },
    )
    .placeholder("Select a deck from the above.");

    let meta_handle = ctx
        .send(
            poise::CreateReply::default()
                .embed(embed.clone())
                .components(vec![serenity::CreateActionRow::SelectMenu(select_menu.clone())]),
        )
        .await?;

    let mut list_handle = None::<poise::ReplyHandle>;

    let replies = decks
        .iter()
        .cloned()
        .map(|(_, deck)| Lazy::new(|| create_deck_reply(deck).unwrap_or_default()))
        .collect::<Vec<_>>();

    while let Some(choice) = serenity::collector::ComponentInteractionCollector::new(ctx)
        .filter(move |choice| choice.data.custom_id.starts_with(&ctx_id.to_string()))
        .timeout(std::time::Duration::from_secs(300)) // 5 minutes
        .await
    {
        let serenity::ComponentInteractionDataKind::StringSelect { ref values } = choice.data.kind
        else {
            continue;
        };

        choice
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::Acknowledge,
            )
            .await?;

        let i = values[0].parse::<usize>()?;
        let reply = replies[i].clone();

        if let Some(list_handle) = list_handle.as_ref() {
            list_handle.edit(ctx, reply).await?;
        } else {
            list_handle = Some(ctx.send(reply).await?);
        }
    }

    meta_handle
        .edit(
            ctx,
            poise::CreateReply::default().embed(embed).components(vec![
                serenity::CreateActionRow::SelectMenu(select_menu.disabled(true)),
            ]),
        )
        .await?;

    Ok(())
}

async fn parse_format(ctx: Context<'_>, format: Option<String>) -> mimiron::deck::Format {
    format
        .or(ctx.guild_channel().await.map(|c| c.name).filter(|n| {
            n.eq_ignore_ascii_case("standard")
                || n.eq_ignore_ascii_case("std")
                || n.eq_ignore_ascii_case("wild")
                || n.eq_ignore_ascii_case("twist")
        })) // clever stuff !! too clever?
        .and_then(|s| s.parse().ok())
        .unwrap_or_default()
}
