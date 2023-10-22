use mimiron::deck::{self, Deck};
use poise::serenity_prelude as serenity;
use std::{borrow::Cow, io::Cursor};

type Error = crate::Error;
type Context<'a> = crate::Context<'a>;

/// Get deck cards from code.
#[poise::command(slash_command)]
pub async fn deck(
    ctx: Context<'_>,
    #[description = "deck code"] code: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let deck = deck::lookup(&code)?;
    /***
     * this code is to list the cards by text. Do I even want that?
     * 
    let cards = order_cards(&deck.cards);

    let mut class_cards_buffer = String::new();
    let mut neutrals_buffer = String::new();

    for (card, count) in cards {
        let (square, count) = square_count(card, count);
        if card.class.contains(&Class::Neutral) {
            writeln!(neutrals_buffer, "{} {}{}", square, count, card.name)?;
        } else {
            writeln!(class_cards_buffer, "{} {}{}", square, count, card.name)?;
        }
    }

    let mut fields = vec![
        (String::from("Class Cards"), class_cards_buffer, true),
        (String::from("Neutrals"), neutrals_buffer, true),
    ];

    if let Some(sideboards) = &deck.sideboard_cards {
        for sb in sideboards {
            let name = format!("{} Sideboard", &sb.sideboard_card.name);
            let cards = order_cards(&sb.cards_in_sideboard);

            let mut sb_buffer = String::new();

            for (card, count) in cards {
                let (square, count) = square_count(card, count);
                writeln!(sb_buffer, "{} {}{}", square, count, card.name)?;
            }

            fields.push((name, sb_buffer, true));
        }
    } */

    let cursor = inner_get_image(&deck)?;

    ctx.send(|reply| {
        reply
            .embed(|embed| {
                embed
                    .title(format!("{} Deck", deck.class))
                    .url(format!(
                        "https://hearthstone.blizzard.com/deckbuilder?deckcode={}",
                        urlencoding::encode(&deck.deck_code)
                    ))
                    .description(code)
                    .color(deck.class.color())
                    // .fields(fields)
                    .attachment("deck.png")
            })
            .attachment(serenity::AttachmentType::Bytes {
                data: Cow::Owned(cursor.into_inner()),
                filename: "deck.png".into(),
            })
    })
    .await?;

    Ok(())
}

fn inner_get_image(deck: &Deck) -> Result<Cursor<Vec<u8>>, anyhow::Error> {
    let opts = deck::ImageOptions::Regular {
        columns: 2,
        with_text: false,
    };
    let img = deck::get_image(&deck, opts)?;

    let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
    img.write_to(&mut cursor, image::ImageOutputFormat::Png)?;

    Ok(cursor)
}

/****
 *  I don't know if I even want to do these. List cards by text or rely on image?
 * 
fn order_cards(cards: &[Card]) -> BTreeMap<&Card, usize> {
    cards.iter().fold(BTreeMap::new(), |mut map, card| {
        *map.entry(card).or_default() += 1;
        map
    })
}

fn square_count(card: &Card, count: usize) -> (&str, String) {
    let square = match card.rarity {
        Rarity::Legendary => ":large_orange_diamond:",
        Rarity::Epic => ":purple_circle:",
        Rarity::Rare => ":small_blue_diamond:",
        _ => ":white_small_square:",
    };

    let count = if count == 1 {
        "   ".into()
    } else {
        format!("{}x ", count)
    };

    (square, count)
} */
