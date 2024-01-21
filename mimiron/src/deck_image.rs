#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::cast_sign_loss
)]

use crate::{
    card::Card,
    card_details::{Class, Rarity},
    deck::Deck,
    localization::{Locale, Localize},
    AGENT,
};
use anyhow::{anyhow, Result};
use futures::{AsyncReadExt, StreamExt, stream};
use image::{imageops, DynamicImage, GenericImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::{
    drawing::{self, Canvas as _},
    pixelops::weighted_sum,
    rect::Rect,
};
use isahc::RequestExt;
use once_cell::sync::Lazy;
use rusttype::{Font, Scale};
use std::collections::{BTreeMap, HashMap};

//  Numbers based on the crops provided by Blizzard API
const CROP_WIDTH: u32 = 243;
const CROP_HEIGHT: u32 = 64;

const MARGIN: u32 = 5;

const SLUG_WIDTH: u32 = CROP_WIDTH * 2 + CROP_HEIGHT;
const ROW_HEIGHT: u32 = CROP_HEIGHT + MARGIN;
const COLUMN_WIDTH: u32 = SLUG_WIDTH + MARGIN;

// fonts unified for all usages now that the Text Box is removed.
static FONTS: [(Lazy<Font<'_>>, f32); 3] = [
    // Base font
    (
        Lazy::new(|| {
            Font::try_from_bytes(include_bytes!("../fonts/YanoneKaffeesatz-Medium.ttf")).unwrap()
        }),
        1.0,
    ),
    // Fallbacks
    (
        Lazy::new(|| {
            Font::try_from_bytes(include_bytes!("../fonts/NotoSansCJK-Medium.ttc")).unwrap()
        }),
        1.2, // scaling for Noto CJK
    ),
    (
        Lazy::new(|| {
            Font::try_from_bytes(include_bytes!("../fonts/NotoSansThaiLooped-Medium.ttf")).unwrap()
        }),
        1.3, // scaling for Noto Thai
    ),
];

#[derive(Clone, Copy)]
pub enum ImageOptions {
    /// Each group in its own column. (HS Top Decks)
    Groups,

    Regular {
        /// 1 is most compact horizontally.
        /// 3 is most compact (yet readable) vertically.
        columns: u8,
    },

    /// Similar to Regular but is either 2 or 3 columns based on "size".
    Adaptable,
}

pub async fn get(deck: &Deck, locale: Locale, shape: ImageOptions) -> Result<DynamicImage> {
    match shape {
        ImageOptions::Groups => img_groups_format(deck, locale).await,
        ImageOptions::Adaptable => img_columns_format(deck, locale, None).await,
        ImageOptions::Regular { columns } => {
            img_columns_format(deck, locale, Some(columns as u32)).await
        }
    }
}

async fn img_columns_format(
    deck: &Deck,
    locale: Locale,
    col_count: Option<u32>,
) -> Result<DynamicImage> {
    let (ordered_cards, slug_map) = order_deck_and_get_slugs(deck).await;

    let (mut img, cards_in_col) = {
        let main_deck_length = ordered_cards.len();

        let sideboards_length = deck.sideboard_cards.as_ref().map_or(0, |sbs| {
            sbs.iter()
                .fold(0, |acc, sb| sb.cards_in_sideboard.len() + 1 + acc)
        });

        let length = (main_deck_length + sideboards_length) as u32;

        // slightly more sophisticated hack for Reno Renathal decks.
        let col_count = col_count.unwrap_or_else(|| (length / 15 + 1).max(2));
        let cards_in_col = length / col_count + (length % col_count).min(1);

        // main canvas
        let img = draw_main_canvas(
            COLUMN_WIDTH * col_count + MARGIN,
            ROW_HEIGHT * (cards_in_col + 1) + MARGIN,
            (255, 255, 255),
        );

        (img, cards_in_col)
    };

    draw_deck_title(&mut img, locale, deck).await?;

    // Main deck
    for (i, (card, _)) in ordered_cards.iter().enumerate() {
        let slug = &slug_map[card];

        let i = i as u32;
        let (col, row) = (i / cards_in_col, i % cards_in_col + 1);

        img.copy_from(slug, col * COLUMN_WIDTH + MARGIN, row * ROW_HEIGHT + MARGIN)?;
    }

    // sideboard cards
    if let Some(sideboards) = &deck.sideboard_cards {
        let mut sb_pos_tracker = ordered_cards.len() as u32;

        for sb in sideboards {
            let (col, row) = (
                sb_pos_tracker / cards_in_col,
                sb_pos_tracker % cards_in_col + 1,
            );
            img.copy_from(
                &get_heading_slug(&format!("> {}", sb.sideboard_card.name)),
                col * COLUMN_WIDTH + MARGIN,
                row * ROW_HEIGHT + MARGIN,
            )?;
            sb_pos_tracker += 1;

            for slug in order_cards(&sb.cards_in_sideboard)
                .keys()
                .map(|c| &slug_map[c])
            {
                let i = sb_pos_tracker;
                let (col, row) = (i / cards_in_col, i % cards_in_col + 1);
                img.copy_from(slug, col * COLUMN_WIDTH + MARGIN, row * ROW_HEIGHT + MARGIN)?;

                sb_pos_tracker += 1;
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(img))
}

async fn img_groups_format(deck: &Deck, locale: Locale) -> Result<DynamicImage> {
    let (ordered_cards, slug_map) = order_deck_and_get_slugs(deck).await;

    let class_cards = ordered_cards
        .iter()
        .filter(|&(c, _)| !c.class.contains(&Class::Neutral))
        .map(|(c, _)| &slug_map[c])
        .enumerate()
        .collect::<Vec<_>>();

    let neutral_cards = ordered_cards
        .iter()
        .filter(|&(c, _)| c.class.contains(&Class::Neutral))
        .map(|(c, _)| &slug_map[c])
        .enumerate()
        .collect::<Vec<_>>();

    // deck image width
    // assumes decks will always have class cards
    let deck_img_width = {
        let mut columns = 1;
        if !neutral_cards.is_empty() {
            columns += 1;
        }
        if let Some(sideboards) = &deck.sideboard_cards {
            columns += sideboards.len();
        }

        columns as u32 * COLUMN_WIDTH + MARGIN
    };

    // deck image height
    // ignores length of sideboards. unlikely to be larger than both class_cards and neutral_cards
    let deck_img_height = {
        let length = 1 + class_cards.len().max(neutral_cards.len()) as u32;
        (length * ROW_HEIGHT) + MARGIN
    };

    // main canvas
    let mut img = draw_main_canvas(deck_img_width, deck_img_height, (255, 255, 255));

    draw_deck_title(&mut img, locale, deck).await?;

    // Doesn't currently accomodate longer deck titles
    if !neutral_cards.is_empty() {
        let neutrals_title = get_heading_slug("Neutrals");
        img.copy_from(&neutrals_title, COLUMN_WIDTH + MARGIN, MARGIN)?;
    }

    // class cards
    for (i, slug) in class_cards {
        let i = i as u32 + 1;
        img.copy_from(slug, MARGIN, i * ROW_HEIGHT + MARGIN)?;
    }

    // neutral cards
    for (i, slug) in neutral_cards {
        let i = i as u32 + 1;
        img.copy_from(slug, COLUMN_WIDTH + MARGIN, i * ROW_HEIGHT + MARGIN)?;
    }

    // sideboard cards
    if let Some(sideboards) = &deck.sideboard_cards {
        for (sb_i, sb) in sideboards.iter().enumerate() {
            // 2 can be assumed here because all one of current sideboard cards are neutral.
            let column_start = COLUMN_WIDTH * (2 + sb_i as u32) + MARGIN;

            img.copy_from(
                &get_heading_slug(&format!("> {}", sb.sideboard_card.name)),
                column_start,
                ROW_HEIGHT + MARGIN,
            )?;

            for (i, slug) in order_cards(&sb.cards_in_sideboard)
                .iter()
                .enumerate()
                .map(|(i, (c, _))| (i, &slug_map[c]))
            {
                let i = i as u32 + 2;
                img.copy_from(slug, column_start, i * ROW_HEIGHT + MARGIN)?;
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(img))
}

async fn get_card_slug(card: &Card, count: usize) -> DynamicImage {
    assert!(count > 0);

    let name = &card.name;

    let r_color = &card.rarity.color();

    let slug_height = CROP_HEIGHT;

    // main canvas
    let mut img = draw_main_canvas(SLUG_WIDTH, slug_height, (10, 10, 10));

    if let Err(e) = draw_crop_image(&mut img, card).await {
        eprint!("Failed to get image of {}: {e}            \r", card.name);
        drawing::draw_filled_rect_mut(
            &mut img,
            Rect::at(CROP_WIDTH as i32, 0).of_size(CROP_WIDTH, CROP_HEIGHT),
            Rgba([r_color.0, r_color.1, r_color.2, 255]),
        );
    }

    // gradient
    let mut gradient = RgbaImage::new(CROP_WIDTH, CROP_HEIGHT);
    imageops::horizontal_gradient(
        &mut gradient,
        &Rgba([10u8, 10, 10, 255]),
        &Rgba([10u8, 10, 10, 0]),
    );
    imageops::overlay(&mut img, &gradient, CROP_WIDTH as i64, 0);

    // size
    let scale = Scale::uniform(40.0);

    // card name
    draw_text(
        &mut img,
        (255, 255, 255),
        CROP_HEIGHT as i32 + 10,
        15,
        scale,
        name,
    );

    // mana square
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(CROP_HEIGHT, CROP_HEIGHT),
        Rgba([60, 109, 173, 255]),
    );

    // card cost
    let cost = card.cost.to_string();
    let (tw, _) = drawing::text_size(scale, &FONTS[0].0, &cost);
    draw_text(
        &mut img,
        (255, 255, 255),
        (CROP_HEIGHT as i32 - tw) / 2,
        15,
        scale,
        &cost,
    );

    // rarity square
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(SLUG_WIDTH as i32 - CROP_HEIGHT as i32, 0).of_size(CROP_HEIGHT, CROP_HEIGHT),
        Rgba([r_color.0, r_color.1, r_color.2, 255]),
    );

    // card count
    let count = match (count, &card.rarity) {
        (1, Rarity::Noncollectible) => String::from("!"),
        (1, Rarity::Legendary) => String::new(),
        _ => count.to_string(),
    };
    let (tw, _) = drawing::text_size(scale, &FONTS[0].0, &count);
    draw_text(
        &mut img,
        (255, 255, 255),
        SLUG_WIDTH as i32 - (CROP_HEIGHT as i32 + tw) / 2,
        15,
        scale,
        &count,
    );

    DynamicImage::ImageRgba8(img)
}

fn order_cards(cards: &[Card]) -> BTreeMap<&Card, usize> {
    cards.iter().fold(BTreeMap::new(), |mut map, card| {
        *map.entry(card).or_default() += 1;
        map
    })
}

async fn order_deck_and_get_slugs(
    deck: &Deck,
) -> (BTreeMap<&Card, usize>, HashMap<&Card, DynamicImage>) {
    let ordered_cards = order_cards(&deck.cards);
    let ordered_sbs_cards = deck
        .sideboard_cards
        .iter()
        .flat_map(|sbs| {
            sbs.iter()
                .flat_map(|sb| order_cards(&sb.cards_in_sideboard))
        })
        .collect::<Vec<_>>();

    let slug_map = stream::iter(
        ordered_cards
            .clone()
            .into_iter()
            .chain(ordered_sbs_cards.into_iter()),
    )
    .then(|(card, count)| async move {
        let slug = get_card_slug(card, count).await;
        (card, slug)
    })
    .collect::<HashMap<_, _>>()
    .await;

    (ordered_cards, slug_map)
}

fn get_heading_slug(heading: &str) -> DynamicImage {
    // main canvas
    let mut img = draw_main_canvas(SLUG_WIDTH, CROP_HEIGHT, (255, 255, 255));

    // size
    let scale = Scale::uniform(50.0);

    let (_, th) = drawing::text_size(scale, &FONTS[0].0, "E");

    draw_text(
        &mut img,
        (10, 10, 10),
        15,
        (CROP_HEIGHT as i32 - th) / 2,
        scale,
        heading, //.to_uppercase(),
    );

    DynamicImage::ImageRgba8(img)
}

fn draw_main_canvas(width: u32, height: u32, color: (u8, u8, u8)) -> RgbaImage {
    let mut img = ImageBuffer::new(width, height);
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(width, height),
        Rgba([color.0, color.1, color.2, 255]),
    );
    img
}

async fn draw_deck_title(img: &mut RgbaImage, locale: Locale, deck: &Deck) -> Result<()> {
    let title = deck.title.clone().unwrap_or_else(|| {
        format!(
            "{} - {}",
            deck.class.in_locale(locale),
            deck.format.to_uppercase()
        )
    });

    // size
    let scale = Scale::uniform(50.0);

    let (_, th) = drawing::text_size(scale, &FONTS[0].0, "E");

    // title
    draw_text(
        img,
        (10, 10, 10),
        MARGIN as i32 + CROP_HEIGHT as i32 + 10,
        MARGIN as i32 + (CROP_HEIGHT as i32 - th) / 2,
        scale,
        &title,
    );

    if let Ok(class_img) = get_class_icon(&deck.class).await {
        img.copy_from(
            &class_img.resize_to_fill(CROP_HEIGHT, CROP_HEIGHT, imageops::FilterType::Gaussian),
            MARGIN,
            MARGIN,
        )?;
    }

    Ok(())
}

async fn get_class_icon(class: &Class) -> Result<DynamicImage> {
    let mut buf = Vec::new();

    let link = url::Url::parse(
        &(format!(
            "https://render.worldofwarcraft.com/us/icons/56/classicon_{}.jpg",
            class
                .in_en_us()
                .to_string()
                .to_ascii_lowercase()
                .replace(' ', "")
        )),
    )?;

    isahc::Request::get(link.as_str())
        .body(())?
        .send_async()
        .await?
        .into_body()
        .read_to_end(&mut buf)
        .await?;

    Ok(image::load_from_memory(&buf)?)
}

async fn draw_crop_image(img: &mut RgbaImage, card: &Card) -> Result<()> {
    let link = card
        .crop_image
        .clone()
        .or_else(|| {
            // refer to: https://hearthstonejson.com/docs/images.html
            crate::card_details::get_hearth_sim_id(card)
                .map(|id| format!("https://art.hearthstonejson.com/v1/tiles/{id}.png"))
        })
        .ok_or_else(|| anyhow!("Card {} has no crop image", card.name))?;

    let mut buf = Vec::new();

    isahc::Request::get(link)
        .body(())?
        .send_async()
        .await?
        .into_body()
        .read_to_end(&mut buf)
        .await?;

    let crop = image::load_from_memory(&buf)?;

    img.copy_from(&crop, CROP_WIDTH, 0)?;

    Ok(())
}

// isolate the function to inline `imageproc::drawing::draw_text_mut` and impl font fallback.
fn draw_text<'a>(
    canvas: &'a mut RgbaImage,
    color: (u8, u8, u8),
    x: i32,
    y: i32,
    scale: Scale,
    text: &'a str,
) {
    let image_width = canvas.width() as i32;
    let image_height = canvas.height() as i32;

    // fonts unified for all usages now that the Text Box is removed.
    let font = &FONTS[0].0;

    let mut caret = 0.0;
    let v_metric = font.v_metrics(scale).ascent;

    for c in text.chars() {
        let Some(g) = FONTS
            .iter()
            .map(|(f_f, f_s)| f_f.glyph(c).scaled(Scale::uniform(scale.x * f_s)))
            .find(|g| g.id().0 > 0)
        else {
            continue;
        };

        let g = g.positioned(rusttype::point(caret, v_metric));

        caret += g.unpositioned().h_metrics().advance_width;

        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|gx, gy, gv| {
                let image_x = gx as i32 + bb.min.x + x;
                let image_y = gy as i32 + bb.min.y + y;

                if (0..image_width).contains(&image_x) && (0..image_height).contains(&image_y) {
                    let pixel = canvas.get_pixel(image_x as u32, image_y as u32).to_owned();
                    let color = Rgba([color.0, color.1, color.2, 255]);
                    let weighted_color = weighted_sum(pixel, color, 1.0 - gv, gv);
                    canvas.draw_pixel(image_x as u32, image_y as u32, weighted_color);
                }
            });
        }
    }
}
