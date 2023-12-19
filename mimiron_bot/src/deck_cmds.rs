use crate::{Context, Error};
use itertools::Itertools;
use mimiron::{
    card,
    card_details::Rarity,
    deck::{self, Deck},
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

    let mut deck = deck::lookup(&code)?;

    if let Some(fmt) = format {
        deck.format = fmt;
    }

    send_deck_reply(ctx, deck).await
}

/// Get deck cards from by right-clicking a message with a deck code.
#[poise::command(context_menu_command = "Get Deck", category = "Deck")]
pub async fn deck_context_menu(
    ctx: Context<'_>,
    #[description = "deck code"] msg: serenity::Message,
) -> Result<(), Error> {
    ctx.defer().await?;

    let deck = deck::lookup(&msg.content)?;

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

    let mut deck = deck::lookup(&code)?;
    deck::add_band(&mut deck, vec![member1, member2, member3])?;

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

    let deck1 = deck::lookup(&code1)?;

    let deck2 = deck::lookup(&code2)?;

    let deckcomp = deck1.compare_with(&deck2);

    let sort_and_set = |map: HashMap<card::Card, usize>| {
        map.into_iter()
            .sorted()
            .map(|(card, count)| {
                // emojis defined on Mimiron Bot Server.
                let square = match card.rarity {
                    Rarity::Legendary => "<:legendary:1182038161099067522>",
                    Rarity::Epic => "<:epic:1182038156841844837>",
                    Rarity::Rare => "<:rare:1182038164781678674>",
                    _ => "<:common:1182038153767419986>",
                };

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
        .title(format!("{} Deck Comparison", deck1.class))
        .color(deck1.class.color())
        .fields(fields);

    let reply = poise::CreateReply::default().embed(embed);

    ctx.send(reply).await?;

    Ok(())
}

async fn send_deck_reply(ctx: Context<'_>, deck: Deck) -> Result<(), Error> {
    let attachment_name = format!("{}s_{}_deck.png", ctx.author().name, deck.class);

    let attachment = {
        let img = deck::get_image(&deck, deck::ImageOptions::Adaptable)?;

        let mut image_data = Cursor::new(Vec::<u8>::new());
        img.write_to(&mut image_data, image::ImageOutputFormat::Png)?;

        serenity::CreateAttachment::bytes(image_data.into_inner(), &attachment_name)
    };

    let embed = serenity::CreateEmbed::new()
        .title(deck.title.unwrap_or(format!("{} Deck", deck.class)))
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
