use crate::{
    helpers::{class_to_emoji, markdown, paginated_card_print, rarity_to_emoji, terse_card_print},
    Context, Error,
};
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

    terse_card_print(ctx, cards, inner_card_embed).await
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

    terse_card_print(ctx, cards, inner_card_embed).await
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

    paginated_card_print(ctx, cards, inner_card_embed).await
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

    terse_card_print(ctx, cards, inner_card_embed).await
}

fn inner_card_embed(card: card::Card) -> serenity::CreateEmbed {
    let class = card
        .class
        .into_iter()
        .map(class_to_emoji)
        .collect::<String>();

    let rarity = rarity_to_emoji(card.rarity.clone());

    let mut fields = vec![
        (
            " ",
            format!("{} {} mana {}", class, card.cost, card.card_type),
            true,
        ),
        (" ", format!("{} {}", rarity, card.card_set.clone()), true),
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
