use mimiron::deck::{self, Deck};
use poise::serenity_prelude as serenity;
use std::{borrow::Cow, io::Cursor};

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Get deck cards from code.
#[poise::command(slash_command)]
pub async fn deck(
    ctx: Context<'_>,
    #[description = "deck code"] code: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let deck = deck::lookup(&code)?;

    send_deck_reply(ctx, deck).await
}

/// Add band to a deck without a band.
#[poise::command(slash_command)]
pub async fn addband(
    ctx: Context<'_>,
    #[description = "deck code"] code: String,
    #[description = "band member"] member1: String,
    #[description = "band member"] member2: String,
    #[description = "band member"] member3: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let mut deck = deck::lookup(&code)?;
    mimiron::deck::add_band(&mut deck, vec![member1, member2, member3])?;

    send_deck_reply(ctx, deck).await
}

fn inner_get_image(deck: &Deck) -> Result<Cursor<Vec<u8>>, anyhow::Error> {
    let img = deck::get_image(&deck, deck::ImageOptions::Adaptable)?;

    let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
    img.write_to(&mut cursor, image::ImageOutputFormat::Png)?;

    Ok(cursor)
}

async fn send_deck_reply(ctx: Context<'_>, deck: Deck) -> Result<(), Error> {
    let cursor = inner_get_image(&deck)?;

    ctx.send(|reply| {
        reply
            .embed(|embed| {
                embed
                    .title(format!("{} Deck", deck.class))
                    .url(format!(
                        "https://hearthstone.blizzard.com/deckbuilder?deckcode={}",
                        urlencoding::encode(&deck.deck_code)
                    ))
                    .description(&deck.deck_code)
                    .color(deck.class.color())
                    .attachment("deck.png")
            })
            .attachment(serenity::AttachmentType::Bytes {
                data: Cow::Owned(cursor.into_inner()),
                filename: "deck.png".into(),
            })
    })
    .await?;

    Ok(())
}
