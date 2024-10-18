use crate::{
    helpers::{get_server_locale, paginated_embeds, terse_embeds, Emoji},
    Context, Error,
};
use mimiron::{
    card, keyword,
    localization::{Locale, Localize},
    CardTextDisplay,
};
use poise::serenity_prelude as serenity;
use std::ops::Not;

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

    paginated_embeds(ctx, cards, |c| inner_card_embed(&c, locale)).await
}

/// Search for a constructed card by name, including reprints. Be precise!
#[poise::command(slash_command, category = "Constructed")]
pub async fn cardreprints(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts =
        card::SearchOptions::search_for(search_term).include_reprints(true).with_locale(locale);
    let cards = card::lookup(&opts)?;

    paginated_embeds(ctx, cards, |c| inner_card_embed(&c, locale)).await
}

/// Search for a constructed card by text.
#[poise::command(slash_command, category = "Constructed")]
pub async fn cardtext(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = card::SearchOptions::search_for(search_term).with_text(true).with_locale(locale);
    let cards = card::lookup(&opts)?;

    paginated_embeds(ctx, cards, |c| inner_card_embed(&c, locale)).await
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

    paginated_embeds(ctx, cards, |c| inner_card_embed(&c, locale)).await
}

fn inner_card_embed(card: &card::Card, locale: Locale) -> serenity::CreateEmbed {
    let class = card.class.iter().map(Emoji::emoji).collect::<String>();

    let rarity = card.rarity.emoji();

    let mut fields = vec![
        (" ", format!("{} ({}) {}", class, card.cost, card.card_type.in_locale(locale)), true),
        (" ", format!("{} {}", rarity, card.card_set(locale)), true),
    ];

    if card.in_arena {
        fields.push((" ", "<:arena:1293955150918189067>".into(), true));
    }

    if card.flavor_text.is_empty().not() {
        fields.push(("Flavor Text", card.flavor_text.to_markdown(), false));
    }

    serenity::CreateEmbed::default()
        .title(&card.name)
        .url(format!("https://hearthstone.blizzard.com/en-us/cards/{}", &card.id))
        .description(card.text.to_markdown())
        .color(card.rarity.color())
        .thumbnail(&card.image)
        .fields(fields)
}

/// Search for a keyword!
#[poise::command(slash_command, category = "Constructed")]
pub async fn keyword(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let kws = keyword::lookup(&search_term)?;

    terse_embeds(ctx, kws, |kw|
        serenity::CreateEmbed::default()
            .title(kw.name(locale))
            .description(kw.text(locale))
            .color(0xDEAD)
    )
    .await
}
