use crate::{helpers::markdown, Context, Error};
use itertools::Itertools;
use mimiron::card;
use poise::serenity_prelude as serenity;

/// Search for a constructed card by name. Be precise!
#[poise::command(slash_command, category = "Constructed")]
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
#[poise::command(slash_command, category = "Constructed")]
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
#[poise::command(slash_command, category = "Constructed")]
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
#[poise::command(slash_command, category = "Constructed")]
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
    let embeds = cards.map(inner_card_embed);

    let mut reply = poise::CreateReply::default();
    reply.embeds.extend(embeds);

    ctx.send(reply).await?;

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

    let embed_chunks = cards
        .map(inner_card_embed)
        .chunks(3)
        .into_iter()
        .map(|c| c.collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let mut current_page = 0;

    let mut reply = poise::CreateReply::default();
    reply.embeds.extend(embed_chunks[current_page].clone());

    if embed_chunks.len() > 1 {
        reply = reply.components(vec![serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(&prev_button_id).label("<"), //  .disabled(true)
            serenity::CreateButton::new(&next_button_id).label(">"),
        ])]);
    }

    ctx.send(reply).await?;

    // Code copied from poise pagination sample with relevant edits. See comments there for explanation
    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        .timeout(std::time::Duration::from_secs(3600 / 4))
        .await
    {
        current_page = if press.data.custom_id.eq(&next_button_id) {
            (current_page + 1).min(embed_chunks.len() - 1)
        } else {
            current_page.saturating_sub(1)
        };

        press
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .embeds(embed_chunks[current_page].clone()),
                ),
            )
            .await?;
    }

    Ok(())
}

fn inner_card_embed(card: card::Card) -> serenity::CreateEmbed {
    let class = card
        .class
        .into_iter()
        .map(crate::helpers::class_to_emoji)
        .collect::<String>();

    let mut fields = vec![
        (
            " ",
            format!("{} mana {}\n{}", card.cost, card.card_type, class),
            true,
        ),
        (" ", card.card_set.clone(), true),
    ];

    if !card.flavor_text.is_empty() {
        fields.push(("Flavor Text", markdown(&card.flavor_text), false));
    }

    serenity::CreateEmbed::default()
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
