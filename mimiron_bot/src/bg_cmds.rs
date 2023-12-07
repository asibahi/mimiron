use std::str::FromStr;

use crate::{helpers::markdown, Context, Error};
use itertools::Itertools;
use mimiron::{bg, card_details::MinionType};
use poise::serenity_prelude as serenity;

/// Search for a battlegrounds card by name. Be precise!
#[poise::command(slash_command, category = "Battlegrounds")]
pub async fn battlegrounds(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let opts = bg::SearchOptions::empty().search_for(Some(search_term));
    let cards = bg::lookup(&opts)?.take(3);

    let embeds = cards.map(inner_card_embed);
    let mut reply = poise::CreateReply::default();

    reply.embeds.extend(embeds);

    ctx.send(reply).await?;

    Ok(())
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

    paginated_card_print(ctx, cards).await
}

/// Search for a battlegrounds card by tier and optionally minion type.
#[poise::command(slash_command, category = "Battlegrounds")]
pub async fn bgtier(
    ctx: Context<'_>,
    #[description = "tier"] tier: u8,
    #[description = "minion type"] minion_type: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let mt = match minion_type {
        Some(s) => Some(MinionType::from_str(&s)?),
        None => None,
    };

    let opts = bg::SearchOptions::empty()
        .with_tier(Some(tier))
        .with_type(mt);
    let cards = bg::lookup(&opts)?;

    paginated_card_print(ctx, cards).await
}

// code copied almost as-is from card_cmds version. Might want to DRY it?
async fn paginated_card_print(
    ctx: Context<'_>,
    cards: impl Iterator<Item = bg::Card>,
) -> Result<(), Error> {
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
            serenity::CreateButton::new(&prev_button_id)
                .label("<")
                .disabled(true),
            serenity::CreateButton::new(&next_button_id)
                .label(format!("2/{} >", embed_chunks.len())),
        ])]);
    }

    ctx.send(reply).await?;

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

        let prev_button = if current_page == 0 {
            serenity::CreateButton::new(&prev_button_id)
                .label("<")
                .disabled(true)
        } else {
            serenity::CreateButton::new(&prev_button_id).label(format!(
                "< {}/{}",
                current_page,
                embed_chunks.len()
            ))
        };

        let next_button = if current_page == embed_chunks.len() - 1 {
            serenity::CreateButton::new(&next_button_id)
                .label(">")
                .disabled(true)
        } else {
            serenity::CreateButton::new(&next_button_id).label(format!(
                "{}/{} >",
                current_page + 2,
                embed_chunks.len()
            ))
        };

        press
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .embeds(embed_chunks[current_page].clone())
                        .components(vec![serenity::CreateActionRow::Buttons(vec![
                            prev_button,
                            next_button,
                        ])]),
                ),
            )
            .await?;
    }

    Ok(())
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
                        let types = minion_types.iter().join("/");
                        format!("{types}")
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
        bg::get_and_print_associated_cards(card.clone())
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
