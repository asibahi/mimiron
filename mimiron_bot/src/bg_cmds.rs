use crate::{
    helpers::{get_server_locale, markdown, paginated_card_print},
    Context, Error,
};
use itertools::Itertools;
use mimiron::{
    bg,
    card_details::{Locale, Localize},
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

    let opts = bg::SearchOptions::empty().search_for(Some(search_term));
    let cards = bg::lookup(&opts)?;

    paginated_card_print(ctx, cards, |c| inner_card_embed(c, locale)).await
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
        .with_text(true);
    let cards = bg::lookup(&opts)?;

    paginated_card_print(ctx, cards, |c| inner_card_embed(c, locale)).await
}

/// Search for a battlegrounds card by tier and optionally minion type.
#[poise::command(slash_command, category = "Battlegrounds")]
pub async fn bgtier(
    ctx: Context<'_>,
    #[description = "tier"] tier: u8,
    #[description = "minion type"] minion_type: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let locale = get_server_locale(&ctx);

    let opts = bg::SearchOptions::empty()
        .with_tier(Some(tier))
        .with_type(minion_type.map(|s| s.parse()).transpose()?);
    let cards = bg::lookup(&opts)?;

    paginated_card_print(ctx, cards, |c| inner_card_embed(c, locale)).await
}

fn inner_card_embed(card: bg::Card, locale: Locale) -> serenity::CreateEmbed {
    let description = match &card.card_type {
        bg::BGCardType::Hero { armor, .. } => format!("Hero with {armor} armor"),
        bg::BGCardType::Minion { text, .. }
        | bg::BGCardType::Spell { text, .. }
        | bg::BGCardType::Quest { text }
        | bg::BGCardType::Reward { text }
        | bg::BGCardType::Anomaly { text }
        | bg::BGCardType::HeroPower { text, .. } => markdown(text),
    };

    let mut fields = match &card.card_type {
        bg::BGCardType::Minion {
            tier,
            attack,
            health,
            minion_types,
            ..
        } => {
            vec![(
                " ",
                format!(
                    "T-{tier} {attack}/{health} {}",
                    if minion_types.is_empty() {
                        "minion".into()
                    } else {
                        minion_types.iter().map(|t| t.in_locale(locale)).join("/")
                    }
                ),
                true,
            )]
        }
        bg::BGCardType::Spell { tier, cost, .. } => {
            vec![(" ", format!("Tier-{tier}, {cost}-Cost Tavern Spell"), true)]
        }
        bg::BGCardType::Quest { .. } => vec![(" ", "Battlegrounds Quest".into(), true)],
        bg::BGCardType::Reward { .. } => vec![(" ", "Battlegrounds Reward".into(), true)],
        bg::BGCardType::Anomaly { .. } => vec![(" ", "Battlegrounds Anomaly".into(), true)],
        bg::BGCardType::Hero { .. } | bg::BGCardType::HeroPower { .. } => vec![],
    };

    fields.extend(
        bg::get_and_print_associated_cards(&card, locale)
            .into_iter()
            .filter_map(|assoc_card| match assoc_card.card_type {
                bg::BGCardType::Minion {
                    tier,
                    attack,
                    health,
                    text,
                    minion_types,
                    ..
                } => {
                    let title = match card.card_type {
                        bg::BGCardType::Minion { .. } => "Triple",
                        bg::BGCardType::Hero { .. } => "Buddy",
                        _ => "UNKNOWN",
                    };

                    let content = format!(
                        "T-{tier} {attack}/{health} {}: {}",
                        // forget about the word "minion" for now. 
                        // Need to get markdown'ed card_info out of the library to add it.
                        minion_types.iter().map(|t| t.in_locale(locale)).join("/"), 
                        markdown(&text)
                    );

                    Some((title, content, false))
                }
                bg::BGCardType::HeroPower { cost, text } => {
                    Some((" ", format!("({cost}): {}", markdown(&text)), false))
                }
                _ => None,
            }),
    );

    serenity::CreateEmbed::default()
        .title(&card.name)
        .url(format!(
            "https://hearthstone.blizzard.com/en-us/battlegrounds/{}",
            card.id
        ))
        .thumbnail(&card.image)
        .description(description)
        .fields(fields)
}
