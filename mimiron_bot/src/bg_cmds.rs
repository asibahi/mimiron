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
            serenity::CreateButton::new(&prev_button_id).label("<"), 
            serenity::CreateButton::new(&next_button_id).label(">"),
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

fn inner_card_embed(card: bg::Card) -> serenity::CreateEmbed {
    serenity::CreateEmbed::default()
        .title(&card.name)
        .url(format!(
            "https://hearthstone.blizzard.com/en-us/battlegrounds/{}",
            card.id
        ))
        .thumbnail(&card.image)
        .field(" ", format_card_type(&card), false)
        .fields(
            bg::get_and_print_associated_cards(card)
                .into_iter()
                .map(|c| {
                    let field_title = match c.card_type {
                        bg::BGCardType::Minion { .. } => "Triple",
                        bg::BGCardType::HeroPower { .. } => "Hero Power",
                        _ => "",
                    };
                    (field_title, format_card_type(&c), false)
                }),
        )
}

fn format_card_type(card: &bg::Card) -> String {
    match &card.card_type {
        bg::BGCardType::Hero { armor, .. } => format!("Hero with {armor} armor"),
        bg::BGCardType::Minion {
            tier,
            attack,
            health,
            text,
            minion_types,
            ..
        } => {
            format!(
                "Tier-{tier} {attack}/{health} {}\n{}",
                if minion_types.is_empty() {
                    format!("minion")
                } else {
                    let types = minion_types.iter().join("/");
                    format!("{types}")
                },
                markdown(text)
            )
        }
        bg::BGCardType::Spell { tier, cost, text } => {
            format!("Tier-{tier}, {cost}-Cost spell: {}", markdown(text))
        }
        bg::BGCardType::Quest { text } => {
            format!("Battlegrounds Quest: {}", markdown(text))
        }
        bg::BGCardType::Reward { text } => {
            format!("Battlegrounds Reward: {}", markdown(text))
        }
        bg::BGCardType::Anomaly { text } => {
            format!("Battlegrounds Anomaly: {}", markdown(text))
        }
        bg::BGCardType::HeroPower { cost, text } => {
            format!("({cost}) Gold: {}", markdown(text))
        }
    }
}
