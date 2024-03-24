#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::cast_sign_loss
)]

use crate::{
    card::Card,
    card_details::{get_hearth_sim_details, CardType, Class, Rarity},
    deck::Deck,
    localization::{Locale, Localize},
    AGENT,
};
use ab_glyph::{Font, FontRef, ScaleFont};
use anyhow::Result;
use image::{imageops, DynamicImage, GenericImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::{
    drawing::{self, Canvas as _},
    pixelops::weighted_sum,
    rect::Rect,
};
use itertools::Itertools;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};

//  Numbers based on the crops provided by Blizzard API
const CROP_WIDTH: u32 = 243;
const CROP_HEIGHT: u32 = 64;

const MARGIN: u32 = 5;

const SLUG_WIDTH: u32 = CROP_WIDTH * 2 + CROP_HEIGHT;
const ROW_HEIGHT: u32 = CROP_HEIGHT + MARGIN;
const COLUMN_WIDTH: u32 = SLUG_WIDTH + MARGIN;

const HEADING_SCALE: f32 = 50.0;
const CARD_NAME_SCALE: f32 = 40.0;

// fonts unified for all usages now that the Text Box is removed.
static FONTS: [(Lazy<FontRef<'_>>, f32); 3] = [
    // Base font
    (
        Lazy::new(|| {
            FontRef::try_from_slice(include_bytes!("../fonts/YanoneKaffeesatz-Medium.ttf")).unwrap()
        }),
        1.0,
    ),
    // Fallbacks
    (
        Lazy::new(|| {
            FontRef::try_from_slice(include_bytes!("../fonts/NotoSansCJK-Medium.ttc")).unwrap()
        }),
        1.2, // scaling for Noto CJK
    ),
    (
        Lazy::new(|| {
            FontRef::try_from_slice(include_bytes!("../fonts/NotoSansThaiLooped-Medium.ttf"))
                .unwrap()
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

pub fn get(deck: &Deck, locale: Locale, shape: ImageOptions) -> Result<DynamicImage> {
    match shape {
        ImageOptions::Groups => img_groups_format(deck, locale),
        ImageOptions::Adaptable => img_columns_format(deck, locale, None),
        ImageOptions::Regular { columns } => img_columns_format(deck, locale, Some(columns as u32)),
    }
}

fn img_columns_format(deck: &Deck, locale: Locale, col_count: Option<u32>) -> Result<DynamicImage> {
    let (ordered_cards, slug_map) = order_deck_and_get_slugs(deck);

    let (mut img, cards_in_col) = {
        let main_deck_length = ordered_cards.len();

        let sideboards_length = deck.sideboard_cards.as_ref().map_or(0, |sbs| {
            sbs.iter().fold(0, |acc, sb| sb.cards_in_sideboard.iter().unique().count() + 1 + acc)
        });

        let length = (main_deck_length + sideboards_length) as u32;

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

    draw_deck_title(&mut img, locale, deck)?;

    // Main deck
    for (i, (card, _)) in ordered_cards.iter().enumerate() {
        let slug = &slug_map[&card.id];

        let i = i as u32;
        let (col, row) = (i / cards_in_col, i % cards_in_col + 1);

        img.copy_from(slug, col * COLUMN_WIDTH + MARGIN, row * ROW_HEIGHT + MARGIN)?;
    }

    // sideboard cards
    if let Some(sideboards) = &deck.sideboard_cards {
        let mut sb_cursor = ordered_cards.len() as u32;

        for sb in sideboards {
            let (col, row) = (sb_cursor / cards_in_col, sb_cursor % cards_in_col + 1);
            img.copy_from(
                &get_heading_slug(&format!("> {}", sb.sideboard_card.name)),
                col * COLUMN_WIDTH + MARGIN,
                row * ROW_HEIGHT + MARGIN,
            )?;
            sb_cursor += 1;

            for slug in order_cards(&sb.cards_in_sideboard).keys().map(|c| &slug_map[&c.id]) {
                let (col, row) = (sb_cursor / cards_in_col, sb_cursor % cards_in_col + 1);
                img.copy_from(slug, col * COLUMN_WIDTH + MARGIN, row * ROW_HEIGHT + MARGIN)?;

                sb_cursor += 1;
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(img))
}

fn img_groups_format(deck: &Deck, locale: Locale) -> Result<DynamicImage> {
    let (ordered_cards, slug_map) = order_deck_and_get_slugs(deck);

    let class_cards = ordered_cards
        .iter()
        .filter(|&(c, _)| !c.class.contains(&Class::Neutral))
        .map(|(c, _)| &slug_map[&c.id])
        .enumerate()
        .collect::<Vec<_>>();

    let neutral_cards = ordered_cards
        .iter()
        .filter(|&(c, _)| c.class.contains(&Class::Neutral))
        .map(|(c, _)| &slug_map[&c.id])
        .enumerate()
        .collect::<Vec<_>>();

    // deck image width
    // assumes decks will always have class cards
    let deck_img_width = {
        let mut columns = 1;
        if !neutral_cards.is_empty() {
            columns += 1;
        }
        if deck.sideboard_cards.is_some() {
            columns += 1;
        }

        columns * COLUMN_WIDTH + MARGIN
    };

    // deck image height
    let deck_img_height = {
        let length = 1 + class_cards.len().max(neutral_cards.len()).max(
            deck.sideboard_cards
                .iter()
                .flatten()
                .fold(0, |acc, sb| acc + (sb.cards_in_sideboard.len() + 1)),
        ) as u32;
        (length * ROW_HEIGHT) + MARGIN
    };

    // main canvas
    let mut img = draw_main_canvas(deck_img_width, deck_img_height, (255, 255, 255));

    draw_deck_title(&mut img, locale, deck)?;

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
        // 2 can be assumed here because all two of current sideboard cards are neutral.
        let sb_col = COLUMN_WIDTH * 2 + MARGIN;
        let mut sb_cursor = 1;

        for sb in sideboards {
            img.copy_from(
                &get_heading_slug(&format!("> {}", sb.sideboard_card.name)),
                sb_col,
                sb_cursor * ROW_HEIGHT + MARGIN,
            )?;

            sb_cursor += 1;

            for slug in order_cards(&sb.cards_in_sideboard).keys().map(|c| &slug_map[&c.id]) {
                img.copy_from(slug, sb_col, sb_cursor * ROW_HEIGHT + MARGIN)?;
                sb_cursor += 1;
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(img))
}

fn get_card_slug(card: &Card, count: usize) -> DynamicImage {
    assert!(count > 0);

    let (name, cost, rarity) = if let Some(Some((name, cost, rarity))) =
        matches!(card.card_type, CardType::Unknown).then(|| get_hearth_sim_details(&card.id))
    {
        (name, cost, rarity)
    } else {
        (card.name.as_str(), card.cost, card.rarity)
    };

    let r_color = rarity.color();

    // main canvas
    let mut img = draw_main_canvas(SLUG_WIDTH, CROP_HEIGHT, (10, 10, 10));

    match get_crop_image(card) {
        Ok(crop) => {
            img.copy_from(&crop, CROP_WIDTH, 0).ok();
        }
        Err(e) => {
            eprintln!("Failed to get image of {name}: {e}.");
            drawing::draw_filled_rect_mut(
                &mut img,
                Rect::at(CROP_WIDTH as i32, 0).of_size(CROP_WIDTH, CROP_HEIGHT),
                Rgba([r_color.0, r_color.1, r_color.2, 255]),
            );
        }
    }

    // gradient
    let mut gradient = RgbaImage::new(CROP_WIDTH, CROP_HEIGHT);
    imageops::horizontal_gradient(
        &mut gradient,
        &Rgba([10u8, 10, 10, 255]),
        &Rgba([10u8, 10, 10, 0]),
    );
    imageops::overlay(&mut img, &gradient, CROP_WIDTH as i64, 0);

    // card name
    draw_text(&mut img, (255, 255, 255), CROP_HEIGHT + 10, 15, CARD_NAME_SCALE, name);

    // mana square
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(CROP_HEIGHT, CROP_HEIGHT),
        Rgba([60, 109, 173, 255]),
    );

    // card cost
    let cost = cost.to_string();
    let (tw, _) = drawing::text_size(CARD_NAME_SCALE, &*FONTS[0].0, &cost);
    draw_text(&mut img, (255, 255, 255), (CROP_HEIGHT - tw) / 2, 15, CARD_NAME_SCALE, &cost);

    // rarity square
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(SLUG_WIDTH as i32 - CROP_HEIGHT as i32, 0).of_size(CROP_HEIGHT, CROP_HEIGHT),
        Rgba([r_color.0, r_color.1, r_color.2, 255]),
    );

    // card count
    let count = match (count, rarity) {
        (1, Rarity::Noncollectible) => String::from("!"),
        (1, Rarity::Legendary) => String::new(),
        _ => count.to_string(),
    };
    let (tw, _) = drawing::text_size(CARD_NAME_SCALE, &*FONTS[0].0, &count);
    draw_text(
        &mut img,
        (255, 255, 255),
        SLUG_WIDTH - (CROP_HEIGHT + tw) / 2,
        15,
        CARD_NAME_SCALE,
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

fn order_deck_and_get_slugs(deck: &Deck) -> (BTreeMap<&Card, usize>, HashMap<usize, DynamicImage>) {
    let ordered_cards = order_cards(&deck.cards);
    let ordered_sbs_cards = deck
        .sideboard_cards
        .iter()
        .flat_map(|sbs| sbs.iter().flat_map(|sb| order_cards(&sb.cards_in_sideboard)))
        .collect::<Vec<_>>();

    // if a card is in two zones it'd have the same slug in both.
    let slug_map = ordered_cards
        .clone()
        .into_par_iter()
        .chain(ordered_sbs_cards.into_par_iter())
        .map(|(card, count)| {
            let slug = get_card_slug(card, count);
            (card.id, slug)
        })
        .collect::<HashMap<_, _>>();

    (ordered_cards, slug_map)
}

fn get_heading_slug(heading: &str) -> DynamicImage {
    // main canvas
    let mut img = draw_main_canvas(SLUG_WIDTH, CROP_HEIGHT, (255, 255, 255));

    // size
    let th = FONTS[0].0.as_scaled(HEADING_SCALE).ascent() as u32;

    draw_text(
        &mut img,
        (10, 10, 10),
        15,
        (CROP_HEIGHT - th) / 2,
        HEADING_SCALE,
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

fn draw_deck_title(img: &mut RgbaImage, locale: Locale, deck: &Deck) -> Result<()> {
    let title = deck.title.clone().unwrap_or_else(|| {
        format!("{} - {}", deck.class.in_locale(locale), deck.format.to_string().to_uppercase())
    });

    // size
    let th = FONTS[0].0.as_scaled(HEADING_SCALE).ascent() as u32;

    // title
    draw_text(
        img,
        (10, 10, 10),
        MARGIN + CROP_HEIGHT + 10,
        MARGIN + (CROP_HEIGHT - th) / 2,
        HEADING_SCALE,
        &title,
    );

    if let Ok(class_img) = get_class_icon(deck.class) {
        img.copy_from(
            &class_img.resize_to_fill(CROP_HEIGHT, CROP_HEIGHT, imageops::FilterType::Gaussian),
            MARGIN,
            MARGIN,
        )?;
    }

    Ok(())
}

#[cached::proc_macro::cached(result = true)]
fn get_class_icon(class: Class) -> Result<DynamicImage> {
    let mut buf = Vec::new();
    AGENT
        .get(
            &(format!(
                "https://render.worldofwarcraft.com/us/icons/56/classicon_{}.jpg",
                class.in_en_us().to_string().to_ascii_lowercase().replace(' ', "")
            )),
        )
        .call()?
        .into_reader()
        .read_to_end(&mut buf)?;

    Ok(image::load_from_memory(&buf)?)
}

#[cached::proc_macro::cached(
    time = 86400, // one day.
    time_refresh = true,
    result = true,
    key = "usize",
    convert = r#"{(card.id)}"#
)]
fn get_crop_image(card: &Card) -> Result<DynamicImage> {
    let link = card
        .crop_image
        .clone()
        .or_else(|| crate::card_details::get_hearth_sim_crop_image(card.id))
        .unwrap_or("https://art.hearthstonejson.com/v1/tiles/GAME_006.png".into());

    let mut buf = Vec::new();
    AGENT.get(&link).call()?.into_reader().read_to_end(&mut buf)?;

    Ok(image::load_from_memory(&buf)?)
}

// isolate the function to inline `imageproc::drawing::draw_text_mut` and impl font fallback.
fn draw_text<'a>(
    canvas: &'a mut RgbaImage,
    color: (u8, u8, u8),
    x: u32,
    y: u32,
    scale: f32,
    text: &'a str,
) {
    let (image_width, image_height) = canvas.dimensions();

    let mut caret = 0.0;
    let v_metric = FONTS[0].0.as_scaled(scale).ascent();

    for c in text.chars() {
        let Some((f_f, f_s)) = FONTS.iter().find(|(f_f, _)| f_f.glyph_id(c).0 > 0) else {
            continue;
        };

        let f_f = f_f.as_scaled(scale * f_s);

        let mut g = f_f.scaled_glyph(c);
        g.position = (caret, v_metric).into();

        caret += f_f.h_advance(g.id);

        let Some(g) = f_f.outline_glyph(g) else {
            continue;
        };

        let bb = g.px_bounds();
        g.draw(|gx, gy, gv| {
            let image_x = gx + bb.min.x as u32 + x;
            let image_y = gy + bb.min.y as u32 + y;

            if (0..image_width).contains(&image_x) && (0..image_height).contains(&image_y) {
                let pixel = canvas.get_pixel(image_x, image_y).to_owned();
                let color = Rgba([color.0, color.1, color.2, 255]);
                let weighted_color = weighted_sum(pixel, color, 1.0 - gv, gv);
                canvas.draw_pixel(image_x, image_y, weighted_color);
            }
        });
    }
}
