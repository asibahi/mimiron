// use crate::markdown;
use itertools::Itertools;
use mimiron::bg;
use poise::serenity_prelude as serenity;

use crate::markdown;

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Search for a battlegrounds card by name. Be precise!
#[poise::command(slash_command, category = "Battlegrounds")]
pub async fn battlegrounds(
    ctx: Context<'_>,
    #[description = "search term"] search_term: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let opts = bg::SearchOptions::empty().search_for(Some(search_term));
    let cards = bg::lookup(&opts)?.take(3);

    let embeds = cards.map(|card| {
        serenity::CreateEmbed::default()
            .title(&card.name)
            .url(format!(
                "https://hearthstone.blizzard.com/en-us/battlegrounds/{}",
                card.id
            ))
            .thumbnail(&card.image)
            .field(" ", format_card_type(&card), false)
    });

    let mut reply = poise::CreateReply::default();
    reply.embeds.extend(embeds);
    // for embed in embeds {
    //     reply = reply.embed(embed);
    // }

    ctx.send(reply).await?;

    Ok(())
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
