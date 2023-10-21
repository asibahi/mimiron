use crate::markdown;
use mimiron::card;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

async fn inner_card_search(
    ctx: Context<'_>,
    cards: impl Iterator<Item = card::Card>,
) -> Result<(), Error> {
    let cards = cards.take(3);

    ctx.send(|reply| {
        for card in cards {
            reply.embed(|embed| {
                embed
                    .title(&card.name)
                    .url(format!(
                        "https://hearthstone.blizzard.com/en-us/cards/{}",
                        &card.id
                    ))
                    .description(markdown(&card.text))
                    .color(card.rarity.color())
                    .thumbnail(&card.image)
                    .field(" ", &card.card_type.to_string(), true)
                    .field(" ", &card.card_set, true)
                    .field("Flavor Text", markdown(&card.flavor_text), false)
            });
        }
        reply
    })
    .await?;
    Ok(())
}

/// Search for a constructed card by name. Be precise!
#[poise::command(slash_command)]
pub async fn card(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    let opts = card::SearchOptions::search_for(search_term);
    let cards = card::lookup(&opts)?;

    inner_card_search(ctx, cards).await?;

    Ok(())
}

/// Search for a constructed card by name, including reprints. Be precise!
#[poise::command(slash_command)]
pub async fn cardreprints(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    let opts = card::SearchOptions::search_for(search_term).include_reprints(true);
    let cards = card::lookup(&opts)?;

    inner_card_search(ctx, cards).await?;

    Ok(())
}

/// Search for a constructed card by text.
#[poise::command(slash_command)]
pub async fn cardtext(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    let opts = card::SearchOptions::search_for(search_term).with_text(true);
    let cards = card::lookup(&opts)?;

    inner_card_search(ctx, cards).await?;

    Ok(())
}
