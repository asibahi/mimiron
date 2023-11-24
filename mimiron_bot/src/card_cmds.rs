use crate::markdown;
use itertools::Itertools;
use mimiron::card;
use poise::serenity_prelude as serenity;

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

    terse_card_print(ctx, cards).await?;

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

    terse_card_print(ctx, cards).await?;

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

    paginated_card_print(ctx, cards).await?;

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

    terse_card_print(ctx, cards).await?;

    Ok(())
}

async fn terse_card_print(
    ctx: Context<'_>,
    cards: impl Iterator<Item = card::Card>,
) -> Result<(), Error> {
    let cards = cards.take(3);

    ctx.send(|reply| {
        for card in cards {
            reply.embed(|embed| inner_card_embed(&card, embed));
        }
        reply
    })
    .await?;

    Ok(())
}

async fn paginated_card_print(
    ctx: Context<'_>,
    cards: impl Iterator<Item = card::Card>,
) -> Result<(), Error> {
    // pagination elements
    let ctx_id = ctx.id();
    let prev_button_id = format!("{ctx_id}prev");
    let next_button_id = format!("{ctx_id}next");

    let card_chunks = cards
        .chunks(3)
        .into_iter()
        .map(|c| c.collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let mut current_page = 0;

    ctx.send(|reply| {
        for card in &card_chunks[current_page] {
            reply.embed(|embed| inner_card_embed(card, embed));
        }

        if card_chunks.len() > 1 {
            reply.components(|component| {
                component.create_action_row(|action_row| {
                    action_row
                        .create_button(|b| b.custom_id(&prev_button_id).label("<"))
                        .create_button(|b| b.custom_id(&next_button_id).label(">"))
                })
            });
        }

        reply
    })
    .await?;

    // Code copied from poise pagination sample with relevant edits. See comments there for explanation
    while let Some(press) = serenity::CollectComponentInteraction::new(ctx)
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        .timeout(std::time::Duration::from_secs(120))
        .await
    {
        current_page = if press.data.custom_id.eq(&next_button_id) {
            (current_page + 1).min(card_chunks.len() - 1)
        } else {
            current_page.saturating_sub(1)
        };

        // Update the message with the new page contents
        press
            .create_interaction_response(ctx, |press_res| {
                press_res
                    .kind(serenity::InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|res_data| {
                        for card in &card_chunks[current_page] {
                            res_data.embed(|embed| inner_card_embed(card, embed));
                        }
                        res_data
                    })
            })
            .await?;
    }

    Ok(())
}

fn inner_card_embed<'e>(
    card: &card::Card,
    embed: &'e mut serenity::CreateEmbed,
) -> &'e mut serenity::CreateEmbed {
    let mut fields = vec![
        (" ", format!("{} mana {}", card.cost, card.card_type), true),
        (" ", card.card_set.clone(), true),
    ];

    if !card.flavor_text.is_empty() {
        fields.push(("Flavor Text", markdown(&card.flavor_text), false));
    }

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
}
