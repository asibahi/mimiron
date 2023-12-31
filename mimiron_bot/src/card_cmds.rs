use crate::{
    helpers::{class_to_emoji, get_server_locale, markdown, paginated_card_print, rarity_to_emoji},
    Context, Error,
};
use mimiron::{
    card,
    card_details::{Locale, Localize},
};
use poise::serenity_prelude as serenity;

/// Search for a constructed card by name. Be precise!
#[poise::command(slash_command, category = "Constructed")]
pub async fn card(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = card::SearchOptions::search_for(search_term).with_locale(locale);
    let cards = card::lookup(&opts)?;

    paginated_card_print(ctx, cards, |c| inner_card_embed(c, locale)).await
}

/// Search for a constructed card by name, including reprints. Be precise!
#[poise::command(slash_command, category = "Constructed")]
pub async fn cardreprints(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = card::SearchOptions::search_for(search_term)
        .include_reprints(true)
        .with_locale(locale);
    let cards = card::lookup(&opts)?;

    paginated_card_print(ctx, cards, |c| inner_card_embed(c, locale)).await
}

/// Search for a constructed card by text.
#[poise::command(slash_command, category = "Constructed")]
pub async fn cardtext(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = card::SearchOptions::search_for(search_term)
        .with_text(true)
        .with_locale(locale);
    let cards = card::lookup(&opts)?;

    paginated_card_print(ctx, cards, |c| inner_card_embed(c, locale)).await
}

/// Search includes all cards, including noncollectibles. Expect some nonsense.
#[poise::command(slash_command, category = "Constructed")]
pub async fn allcards(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = card::SearchOptions::search_for(search_term)
        .include_noncollectibles(true)
        .with_locale(locale);
    let cards = card::lookup(&opts)?;

    paginated_card_print(ctx, cards, |c| inner_card_embed(c, locale)).await
}

fn inner_card_embed(card: card::Card, locale: Locale) -> serenity::CreateEmbed {
    let class = card
        .class
        .into_iter()
        .map(class_to_emoji)
        .collect::<String>();

    let rarity = rarity_to_emoji(card.rarity.clone());

    let mut fields = vec![
        (
            " ",
            format!(
                "{} ({}) {}",
                class,
                card.cost,
                card.card_type.in_locale(locale)
            ),
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
