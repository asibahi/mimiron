use crate::{helpers::get_server_locale, Context, Error};
use itertools::Itertools;
use mimiron::{
    card,
    deck::{self, Deck, LookupOptions},
    localization::{Locale, Localize},
};
use poise::serenity_prelude as serenity;
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

    let opts = LookupOptions::lookup(code).with_locale(locale);

    let mut deck = deck::lookup(&opts)?;

    if let Some(fmt) = format {
        deck.format = fmt;
    }

    send_deck_reply(ctx, deck, locale).await
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

    send_deck_reply(ctx, deck, locale).await
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
                let square = crate::helpers::rarity_to_emoji(card.rarity);
                let count = (count > 1)
                    .then(|| format!("_{count}x_ "))
                    .unwrap_or_default();

                format!("{} {}{}\n", square, count, card.name)
            })
            .collect::<String>()
    };

    let uniques_1 = sort_and_set(deckcomp.deck1_uniques);
    let uniques_2 = sort_and_set(deckcomp.deck2_uniques);
    let shared = sort_and_set(deckcomp.shared_cards);

    let fields = vec![
        (
            deck1.title.as_deref().unwrap_or("Code 1"),
            deck1.deck_code,
            false,
        ),
        (
            deck2.title.as_deref().unwrap_or("Code 2"),
            deck2.deck_code,
            false,
        ),
        (deck1.title.as_deref().unwrap_or("Deck 1"), uniques_1, true),
        (deck2.title.as_deref().unwrap_or("Deck 2"), uniques_2, true),
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

async fn send_deck_reply(ctx: Context<'_>, deck: Deck, locale: Locale) -> Result<(), Error> {
    let attachment_name = "mimiron_deck.png";

    let attachment = {
        let img = deck::get_image(&deck, locale, deck::ImageOptions::Adaptable)?;

        let mut image_data = Cursor::new(Vec::<u8>::new());
        img.write_to(&mut image_data, image::ImageOutputFormat::Png)?;

        serenity::CreateAttachment::bytes(image_data.into_inner(), attachment_name)
    };

    let embed = serenity::CreateEmbed::new()
        .title(
            deck.title
                .unwrap_or(format!("{} Deck", deck.class.in_locale(locale))),
        )
        .url(format!(
            "https://hearthstone.blizzard.com/deckbuilder?deckcode={}",
            urlencoding::encode(&deck.deck_code)
        ))
        .description(&deck.deck_code)
        .color(deck.class.color())
        .attachment(attachment_name);

    let reply = poise::CreateReply::default()
        .attachment(attachment)
        .embed(embed);

    ctx.send(reply).await?;

    Ok(())
}
