use crate::markdown;
use mimiron::card;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Search for a constructed card by name. Be precise!
#[poise::command(slash_command)]
pub async fn card(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let opts = card::SearchOptions::search_for(search_term);
    let cards = card::lookup(&opts)?;

    inner_card_print(ctx, cards).await?;

    Ok(())
}

/// Search for a constructed card by name, including reprints. Be precise!
#[poise::command(slash_command)]
pub async fn cardreprints(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let opts = card::SearchOptions::search_for(search_term).include_reprints(true);
    let cards = card::lookup(&opts)?;

    inner_card_print(ctx, cards).await?;

    Ok(())
}

/// Search for a constructed card by text.
#[poise::command(slash_command)]
pub async fn cardtext(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let opts = card::SearchOptions::search_for(search_term).with_text(true);
    let cards = card::lookup(&opts)?;

    inner_card_print(ctx, cards).await?;

    Ok(())
}

/// Search includes all cards, including noncollectibles. Expect some nonsense.
#[poise::command(slash_command)]
pub async fn allcards(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let opts = card::SearchOptions::search_for(search_term).include_noncollectibles(true);
    let cards = card::lookup(&opts)?;

    inner_card_print(ctx, cards).await?;

    Ok(())
}

async fn inner_card_print(
    ctx: Context<'_>,
    cards: impl Iterator<Item = card::Card>,
) -> Result<(), Error> {
    let cards = cards.take(3);

    ctx.send(|reply| {
        for card in cards {
            let mut fields = vec![
                (" ", format!("{} mana {}", card.cost, card.card_type), true),
                (" ", card.card_set, true),
            ];

            if !card.flavor_text.is_empty() {
                fields.push(("Flavor Text", markdown(&card.flavor_text), false));
            }

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
                    .fields(fields)
            });
        }
        reply
    })
    .await?;
    Ok(())
}
