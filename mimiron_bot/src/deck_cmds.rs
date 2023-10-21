use mimiron::deck;
use poise::serenity_prelude as serenity;
use std::borrow::Cow;
// use crate::markdown;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Search for a constructed card by name. Be precise!
#[poise::command(slash_command)]
pub async fn deck(
    ctx: Context<'_>,
    #[description = "search term"] code: String,
) -> Result<(), Error> {
    let deck = deck::lookup(&code)?;
    let opts = deck::ImageOptions::Regular {
        columns: 3,
        with_text: false,
    };
    let img = deck::get_image(&deck, opts)?;

    ctx.send(|reply| {
        reply.attachment(serenity::AttachmentType::Bytes {
            data: Cow::Borrowed(img.as_bytes()),
            filename: "deck.png".into(),
        })
    })
    .await?;

    Ok(())
}
