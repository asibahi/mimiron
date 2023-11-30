use crate::{Context, Error};
use mimiron::deck::{self, Deck};
use poise::serenity_prelude as serenity;
use std::io::Cursor;

/// Get deck cards from code.
#[poise::command(slash_command, category = "Deck")]
pub async fn deck(
    ctx: Context<'_>,
    #[description = "deck code"] code: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let _title = code
        .strip_prefix("###")
        .and_then(|s| s.split_once("#"))
        .map(|(s, _)| s.trim());

    let code = code
        .split_ascii_whitespace()
        .find(|s| s.starts_with("AA"))
        .unwrap_or(&code);

    let deck = deck::lookup(&code)?;

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
