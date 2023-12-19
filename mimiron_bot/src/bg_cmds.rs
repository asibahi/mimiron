use crate::{
    helpers::{markdown, paginated_card_print, terse_card_print},
    Context, Error,
};
use itertools::Itertools;
use mimiron::bg;
use poise::serenity_prelude as serenity;

/// Search for a battlegrounds card by name. Be precise!
#[poise::command(slash_command, category = "Battlegrounds")]
pub async fn battlegrounds(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let opts = bg::SearchOptions::empty().search_for(Some(search_term));
    let cards = bg::lookup(&opts)?;

    terse_card_print(ctx, cards, inner_card_embed).await
}

/// Search for a battlegrounds card by text.
#[poise::command(slash_command, category = "Battlegrounds")]
pub async fn bgtext(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let opts = bg::SearchOptions::empty()
        .search_for(Some(search_term))
        .with_text(true);
    let cards = bg::lookup(&opts)?;

    paginated_card_print(ctx, cards, inner_card_embed).await
}

/// Search for a battlegrounds card by tier and optionally minion type.
#[poise::command(slash_command, category = "Battlegrounds")]
pub async fn bgtier(
    ctx: Context<'_>,
    #[description = "tier"] tier: u8,
    #[description = "minion type"] minion_type: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let opts = bg::SearchOptions::empty()
        .with_tier(Some(tier))
        .with_type(minion_type.map(|s| s.parse()).transpose()?);
    let cards = bg::lookup(&opts)?;

    paginated_card_print(ctx, cards, inner_card_embed).await
}

fn inner_card_embed(card: bg::Card) -> serenity::CreateEmbed {
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
                    "Tier-{tier} {attack}/{health} {}",
                    if minion_types.is_empty() {
                        "minion".into()
                    } else {
                        minion_types.iter().join("/")
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
        bg::get_and_print_associated_cards(&card)
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
                        "Tier-{tier} {attack}/{health} {}: {}",
                        if minion_types.is_empty() {
                            "minion".into()
                        } else {
                            minion_types.iter().join("/")
                        },
                        markdown(&text)
                    );

                    Some((title, content, false))
                }
                bg::BGCardType::HeroPower { cost, text } => Some((
                    "Hero Power",
                    format!("{cost}-Cost: {}", markdown(&text)),
                    false,
                )),
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
