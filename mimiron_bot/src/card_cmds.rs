use crate::{
    Context, Error,
    helpers::{Emoji, get_server_locale, paginated_embeds, terse_embeds},
};
use mimiron::{
    CardTextDisplay, card, keyword,
    localization::{Locale, Localize},
};
use poise::serenity_prelude as serenity;
use std::ops::Not;

/// Search by name for a constructed card
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    category = "Constructed"
)]
pub async fn card(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = card::SearchOptions::search_for(&search_term).with_locale(locale);
    let cards = card::lookup(opts)?;

    paginated_embeds(ctx, cards, |c| inner_card_embed(&c, locale)).await
}

/// Search by name for a constructed card, includes reprints
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    category = "Constructed"
)]
pub async fn cardreprints(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = card::SearchOptions::search_for(&search_term)
        .include_reprints(true)
        .with_locale(locale);
    let cards = card::lookup(opts)?;

    paginated_embeds(ctx, cards, |c| inner_card_embed(&c, locale)).await
}

/// Search by text for a constructed card
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    category = "Constructed"
)]
pub async fn cardtext(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = card::SearchOptions::search_for(&search_term)
        .with_text(true)
        .with_locale(locale);
    let cards = card::lookup(opts)?;

    paginated_embeds(ctx, cards, |c| inner_card_embed(&c, locale)).await
}

/// Search includes all cards, including tokens. Expect nonsense.
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    category = "Constructed"
)]
pub async fn allcards(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = card::SearchOptions::search_for(&search_term)
        .include_noncollectibles(true)
        .with_locale(locale);
    let cards = card::lookup(opts)?;

    paginated_embeds(ctx, cards, |c| inner_card_embed(&c, locale)).await
}

fn inner_card_embed(
    card: &card::Card,
    locale: Locale,
) -> serenity::CreateEmbed {
    let desc = format!(
        "{} ({}) {}{}",
        card.class.iter().map(Emoji::emoji).collect::<String>(),
        card.cost,
        card.faction
            .map(|f| f.in_locale(locale).to_string())
            .map_or(String::new(), |f| format!("{f} ")),
        card.card_type.in_locale(locale)
    );

    let mut fields = vec![
        (" ", desc, true),
        (
            " ",
            format!("{} {}", card.rarity.emoji(), card.card_set(locale)),
            true,
        ),
    ];

    if card.flavor_text.is_empty().not() {
        fields.push(("Flavor Text", card.flavor_text.to_markdown(), false));
    }

    serenity::CreateEmbed::default()
        .title(&*card.name)
        .url(format!(
            "https://hearthstone.blizzard.com/en-us/cards/{}",
            &card.id
        ))
        .description(card.text.to_markdown())
        .color(card.rarity.color())
        .thumbnail(&*card.image)
        .fields(fields)
}

/// Search for a keyword
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    category = "Constructed"
)]
pub async fn keyword(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let kws = keyword::lookup(&search_term)?;

    terse_embeds(ctx, 3, kws, |kw| {
        serenity::CreateEmbed::default()
            .title(kw.name(locale))
            .description(kw.text(locale))
            .color(0x_DEAD /*GAME*/)
    })
    .await
}
