use crate::{
    helpers::{get_server_locale, paginated_card_print},
    Context, Error,
};
use mimiron::{
    bg,
    localization::{Locale, Localize},
    CardTextDisplay,
};
use poise::serenity_prelude as serenity;

/// alias for /bg
#[poise::command(slash_command, hide_in_help)]
pub async fn battlegrounds(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    bg_inner(ctx, search_term).await
}

/// Search for a battlegrounds card by name. Be precise!
#[poise::command(slash_command, category = "Battlegrounds")]
pub async fn bg(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    bg_inner(ctx, search_term).await
}

pub async fn bg_inner(ctx: Context<'_>, search_term: String) -> Result<(), Error> {
    let locale = get_server_locale(&ctx);

    let opts = bg::SearchOptions::empty().search_for(Some(search_term)).with_locale(locale);
    let cards = bg::lookup(&opts)?;

    paginated_card_print(ctx, cards, |c| inner_card_embed(&c, locale)).await
}

/// Search for a battlegrounds card by text.
#[poise::command(slash_command, category = "Battlegrounds")]
pub async fn bgtext(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = bg::SearchOptions::empty()
        .search_for(Some(search_term))
        .with_locale(locale)
        .with_text(true);
    let cards = bg::lookup(&opts)?;

    paginated_card_print(ctx, cards, |c| inner_card_embed(&c, locale)).await
}

/// Search for a battlegrounds card by tier and optionally minion type.
#[poise::command(slash_command, category = "Battlegrounds")]
pub async fn bgtier(
    ctx: Context<'_>,
    #[description = "tier"]
    #[choices(1, 2, 3, 4, 5, 6, 7)]
    tier: u8,
    #[description = "minion type"]
    #[autocomplete = "autocomplete_type"]
    minion_type: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = bg::SearchOptions::empty()
        .with_tier(Some(tier))
        .with_locale(locale)
        .with_type(minion_type.map(|s| s.parse()).transpose()?);
    let cards = bg::lookup(&opts)?;

    paginated_card_print(ctx, cards, |c| inner_card_embed(&c, locale)).await
}

#[allow(clippy::unused_async)]
// Should probably get a list from the library for ome source of truth. Needs streams.
async fn autocomplete_type<'a>(_: Context<'_>, partial: &'a str) -> impl Iterator<Item = &'a str> {
    [
        "Beast",
        "Demon",
        "Dragon",
        "Elemental",
        "Mech",
        "Murloc",
        "Naga",
        "Pirate",
        "Quilboar",
        "Undead",
    ]
    .into_iter()
    .filter(move |s| s.to_lowercase().starts_with(&partial.to_lowercase()))
}

fn inner_card_embed(card: &bg::Card, locale: Locale) -> serenity::CreateEmbed {
    let lct = card.card_type.in_locale(locale).to_string();
    let (description, mut fields) = match &card.card_type {
        bg::BGCardType::Hero { .. } => (lct, vec![]),
        bg::BGCardType::Minion { text, .. }
        | bg::BGCardType::Spell { text, .. }
        | bg::BGCardType::Quest { text }
        | bg::BGCardType::Reward { text }
        | bg::BGCardType::Anomaly { text } => (text.to_markdown(), vec![(" ".into(), lct, true)]),
        bg::BGCardType::HeroPower { text, .. } => (text.to_markdown(), vec![]),
    };

    fields.extend(bg::get_and_print_associated_cards(card, locale).into_iter().filter_map(
        |assoc_card| {
            let lct = assoc_card.card_type.in_locale(locale);
            match &assoc_card.card_type {
                bg::BGCardType::Minion { text, .. } => {
                    let title = match card.card_type {
                        bg::BGCardType::Hero { .. } => assoc_card.name,
                        bg::BGCardType::Minion { .. } => format!("3x {}", assoc_card.name),
                        _ => " ".into(),
                    };

                    Some((title, format!("{}: {}", lct, text.to_markdown()), false))
                }
                bg::BGCardType::HeroPower { text, .. } => {
                    Some((assoc_card.name, format!("{}: {}", lct, text.to_markdown()), false))
                }
                _ => None,
            }
        },
    ));

    serenity::CreateEmbed::default()
        .title(&card.name)
        .url(format!("https://hearthstone.blizzard.com/en-us/battlegrounds/{}", card.id))
        .thumbnail(&card.image)
        .description(description)
        .fields(fields)
}
