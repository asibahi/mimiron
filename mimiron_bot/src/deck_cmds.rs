use crate::{Context, Error};
use itertools::Itertools;
use mimiron::{
    card,
    card_details::Rarity,
    deck::{self, Deck},
};
use poise::serenity_prelude as serenity;
use std::io::Cursor;

/// Get deck cards from code.
#[poise::command(slash_command, category = "Deck")]
pub async fn deck(
    ctx: Context<'_>,
    #[description = "deck code"] code: String,
    #[description = "mode"] format: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let code = get_code_from_msg(&code).await;
    let mut deck = deck::lookup(code)?;

    if let Some(fmt) = format {
        deck.format = fmt;
    }

    send_deck_reply(ctx, deck).await
}

/// Get deck cards from code.
#[poise::command(context_menu_command = "Get Deck", category = "Deck")]
pub async fn deck_context_menu(
    ctx: Context<'_>,
    #[description = "deck code"] msg: serenity::Message,
) -> Result<(), Error> {
    ctx.defer().await?;

    let code = get_code_from_msg(&msg.content).await;
    let deck = deck::lookup(code)?;

    send_deck_reply(ctx, deck).await
}

/// Add band to a deck without a band.
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

/// Get deck cards from code.
#[poise::command(slash_command, category = "Deck")]
pub async fn deckcomp(
    ctx: Context<'_>,
    #[description = "deck 1 code"] code1: String,
    #[description = "deck 2 code"] code2: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let code1 = get_code_from_msg(&code1).await;
    let deck1 = deck::lookup(code1)?;

    let code2 = get_code_from_msg(&code2).await;
    let deck2 = deck::lookup(code2)?;

    let deckcomp = deck1.compare_with(&deck2);

    let uniques_1 = deckcomp
        .deck1_uniques
        .into_iter()
        .sorted()
        .map(|(card, count)| {
            let (square, count) = square_count(&card, count);
            format!("{} {}{}\n", square, count, card.name)
        })
        .collect::<String>();

    let uniques_2 = deckcomp
        .deck2_uniques
        .into_iter()
        .sorted()
        .map(|(card, count)| {
            let (square, count) = square_count(&card, count);
            format!("{} {}{}\n", square, count, card.name)
        })
        .collect::<String>();

    let shared = deckcomp
        .shared_cards
        .into_iter()
        .sorted()
        .map(|(card, count)| {
            let (square, count) = square_count(&card, count);
            format!("{} {}{}\n", square, count, card.name)
        })
        .collect::<String>();

    let fields = vec![
        ("Code 1", code1, false),
        ("Code 2", code2, false),
        ("Deck 1", &uniques_1, true),
        ("Deck 2", &uniques_2, true),
        ("Shared", &shared, true),
    ];

    let embed = serenity::CreateEmbed::default()
        .title(format!("{} Deck Comparison", deck1.class))
        .color(deck1.class.color())
        .fields(fields);

    let reply = poise::CreateReply::default().embed(embed);

    ctx.send(reply).await?;

    Ok(())
}

async fn get_code_from_msg(code: &str) -> &str {
    /* For when someone pastes something like this:
     * ### Custom Shaman
     * # etc
     * #
     * AAECAfWfAwjy3QT0oAXmowXipAXFpQX9xAX0yAX00AUL1bIE4LUEssEExc4Exs4Euu0Eyu0EhaoFw9AFxNAFr9EFAAED2aAE/cQFr8MF/cQF0p4G/cQFAAA=
     * #
     * # To use this deck, copy it to your clipboard and create a new deck in Hearthstone
     */

    // use this later?
    let _title = code
        .strip_prefix("###")
        .and_then(|s| s.split_once("#"))
        .map(|(s, _)| s.trim());

    code.split_ascii_whitespace()
        .find(|s| s.starts_with("AA"))
        .unwrap_or(&code)
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
        .title(format!("{} Deck", deck.class))
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

fn square_count(card: &card::Card, count: usize) -> (&str, String) {
    // emojis defined on Mimiron Bot Server.
    let square = match card.rarity {
        Rarity::Legendary => "<:legendary:1182038161099067522>",
        Rarity::Epic => "<:epic:1182038156841844837>",
        Rarity::Rare => "<:rare:1182038164781678674>",
        _ => "<:common:1182038153767419986>",
    };

    let count = (count > 1)
        .then(|| format!("{}x ", count))
        .unwrap_or_default();

    (square, count)
}
