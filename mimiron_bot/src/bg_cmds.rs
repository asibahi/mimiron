// use crate::markdown;
use itertools::Itertools;
use mimiron::bg;

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
    let opts = mimiron::bg::SearchOptions::empty().search_for(Some(search_term));
    let cards = mimiron::bg::lookup(&opts)?.take(3);

    ctx.send(|reply| {
        for card in cards {
            reply.embed(|embed| {
                let text = format_card_type(&card);
                embed
                    .title(&card.name)
                    .url(format!(
                        "https://hearthstone.blizzard.com/en-us/battlegrounds/{}",
                        card.id
                    ))
                    .thumbnail(&card.image)
                    .field(" ", text, false);

                for associated_card in bg::get_and_print_associated_cards(card) {
                    let field_title = match associated_card.card_type {
                        bg::BGCardType::Minion { .. } => "Golden",
                        bg::BGCardType::HeroPower { .. } => "Hero Power",
                        _ => "",
                    };
                    embed.field(field_title, format_card_type(&associated_card), false);
                }
                embed
            });
        }
        reply
    })
    .await?;
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
