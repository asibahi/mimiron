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
    localization::Localize,
    AGENT,
};
use ab_glyph::{Font, FontRef, ScaleFont};
use anyhow::Result;
use image::{imageops, GenericImage, Rgba, RgbaImage};
use imageproc::{
    drawing::{self, Canvas as _},
    pixelops::interpolate,
    rect::Rect,
};
use itertools::Itertools;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use std::collections::HashMap;

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

pub fn get(deck: &Deck, shape: ImageOptions) -> Result<RgbaImage> {
    match shape {
        ImageOptions::Groups => img_groups_format(deck),
        ImageOptions::Adaptable => img_columns_format(deck, None),
        ImageOptions::Regular { columns } => img_columns_format(deck, Some(columns as u32)),
    }
}

fn img_columns_format(deck: &Deck, col_count: Option<u32>) -> Result<RgbaImage> {
    let ordered_main_deck = deck.cards.iter().sorted().dedup();

    let (mut img, cards_in_col) = {
        let main_deck_length = ordered_main_deck.clone().count();

        let sideboards_length = deck.sideboard_cards.as_ref().map_or(0, |sbs| {
            sbs.iter().fold(0, |acc, sb| sb.cards_in_sideboard.iter().unique().count() + 1 + acc)
        });

        let length = (main_deck_length + sideboards_length) as u32;

        let col_count = col_count.unwrap_or_else(|| (length / 15 + 1).max(2));
        let cards_in_col = length / col_count + (length % col_count).min(1);

        let img = draw_main_canvas(
            COLUMN_WIDTH * col_count + MARGIN,
            ROW_HEIGHT * (cards_in_col + 1) + MARGIN,
            [255; 4],
        );

        (img, cards_in_col)
    };

    draw_deck_title(&mut img, deck)?;
    let slug_map = get_cards_slugs(deck);

    let mut cursor = 0;

    for card in ordered_main_deck {
        let slug = &slug_map[&(card.id, Zone::MainDeck)];

        let (col, row) = (cursor / cards_in_col, cursor % cards_in_col + 1);

        img.copy_from(slug, col * COLUMN_WIDTH + MARGIN, row * ROW_HEIGHT + MARGIN)?;

        cursor += 1;
    }

    if let Some(sideboards) = &deck.sideboard_cards {
        for sb in sideboards {
            let (col, row) = (cursor / cards_in_col, cursor % cards_in_col + 1);
            img.copy_from(
                &draw_heading_slug(&format!("> {}", sb.sideboard_card.name)),
                col * COLUMN_WIDTH + MARGIN,
                row * ROW_HEIGHT + MARGIN,
            )?;
            cursor += 1;

            for slug in
                sb.cards_in_sideboard.iter().sorted().dedup().map(|c| {
                    &slug_map[&(c.id, Zone::Sideboard { sb_card_id: sb.sideboard_card.id })]
                })
            {
                let (col, row) = (cursor / cards_in_col, cursor % cards_in_col + 1);
                img.copy_from(slug, col * COLUMN_WIDTH + MARGIN, row * ROW_HEIGHT + MARGIN)?;

                cursor += 1;
            }
        }
    }

    Ok(img)
}

fn img_groups_format(deck: &Deck) -> Result<RgbaImage> {
    let ordered_main_deck = deck.cards.iter().sorted().dedup();
    let slug_map = get_cards_slugs(deck);

    let class_cards = ordered_main_deck
        .clone()
        .filter(|&c| !c.class.contains(&Class::Neutral))
        .map(|c| &slug_map[&(c.id, Zone::MainDeck)])
        .enumerate()
        .collect::<Vec<_>>();

    let neutral_cards = ordered_main_deck
        .filter(|&c| c.class.contains(&Class::Neutral))
        .map(|c| &slug_map[&(c.id, Zone::MainDeck)])
        .enumerate()
        .collect::<Vec<_>>();

    let mut img = {
        // assumes decks will always have class cards
        let mut columns = 1;
        if !neutral_cards.is_empty() {
            columns += 1;
        }
        if deck.sideboard_cards.is_some() {
            columns += 1;
        }

        let rows = 1 + class_cards.len().max(neutral_cards.len()).max(
            deck.sideboard_cards
                .iter()
                .flatten()
                .fold(0, |acc, sb| acc + (sb.cards_in_sideboard.iter().unique().count() + 1)),
        ) as u32;

        draw_main_canvas(columns * COLUMN_WIDTH + MARGIN, rows * ROW_HEIGHT + MARGIN, [255; 4])
    };

    draw_deck_title(&mut img, deck)?;

    for (i, slug) in class_cards {
        let i = i as u32 + 1;
        img.copy_from(slug, MARGIN, i * ROW_HEIGHT + MARGIN)?;
    }

    for (i, slug) in neutral_cards {
        let i = i as u32 + 1;
        img.copy_from(slug, COLUMN_WIDTH + MARGIN, i * ROW_HEIGHT + MARGIN)?;
    }

    if let Some(sideboards) = &deck.sideboard_cards {
        // always last column
        let sb_col = img.width() - COLUMN_WIDTH - MARGIN;
        let mut sb_cursor = 1;

        for sb in sideboards {
            img.copy_from(
                &draw_heading_slug(&format!("> {}", sb.sideboard_card.name)),
                sb_col,
                sb_cursor * ROW_HEIGHT + MARGIN,
            )?;
            sb_cursor += 1;

            for slug in
                sb.cards_in_sideboard.iter().sorted().dedup().map(|c| {
                    &slug_map[&(c.id, Zone::Sideboard { sb_card_id: sb.sideboard_card.id })]
                })
            {
                img.copy_from(slug, sb_col, sb_cursor * ROW_HEIGHT + MARGIN)?;
                sb_cursor += 1;
            }
        }
    }

    Ok(img)
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
enum Zone {
    MainDeck,
    Sideboard { sb_card_id: usize },
}

fn draw_card_slug(card: &Card, count: usize, zone: Zone) -> RgbaImage {
    assert!(count > 0);
    _ = zone; // unused for now

    let (name, cost, rarity) = if let Some(Some((name, cost, rarity))) =
        matches!(card.card_type, CardType::Unknown).then(|| get_hearth_sim_details(&card.id))
    {
        (name, cost, rarity)
    } else {
        (card.name.as_str(), card.cost, card.rarity)
    };

    let r_color = rarity.color();

    // main canvas
    let mut img = draw_main_canvas(SLUG_WIDTH, CROP_HEIGHT, [10, 10, 10, 255]);

    match get_crop_image(card) {
        Ok(crop) => {
            img.copy_from(&crop, CROP_WIDTH, 0).ok();

            let mut gradient = RgbaImage::new(CROP_WIDTH, CROP_HEIGHT);
            imageops::horizontal_gradient(
                &mut gradient,
                &Rgba([10u8, 10, 10, 255]),
                &Rgba([10u8, 10, 10, 0]),
            );
            imageops::overlay(&mut img, &gradient, CROP_WIDTH as i64, 0);
        }
        Err(e) => {
            eprintln!("Failed to get image of {name}: {e}.");
            imageops::horizontal_gradient(
                &mut *imageops::crop(&mut img, CROP_WIDTH, 0, CROP_WIDTH, CROP_HEIGHT),
                &Rgba([10u8, 10, 10, 255]),
                &Rgba([r_color.0, r_color.1, r_color.2, 255]),
            );
        }
    }

    // card name
    draw_text(&mut img, [255; 4], CROP_HEIGHT + 10, CARD_NAME_SCALE, name);

    // mana square
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(CROP_HEIGHT, CROP_HEIGHT),
        Rgba([60, 109, 173, 255]),
    );

    // card cost
    let cost = cost.to_string();
    let (tw, _) = drawing::text_size(CARD_NAME_SCALE, &*FONTS[0].0, &cost);
    draw_text(&mut img, [255; 4], (CROP_HEIGHT - tw) / 2, CARD_NAME_SCALE, &cost);

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
    draw_text(&mut img, [255; 4], SLUG_WIDTH - (CROP_HEIGHT + tw) / 2, CARD_NAME_SCALE, &count);

    img
}

fn get_cards_slugs(deck: &Deck) -> HashMap<(usize, Zone), RgbaImage> {
    deck.cards
        .iter()
        .sorted()
        .dedup_with_count()
        .map(|(count, card)| (card, count, Zone::MainDeck))
        .chain(deck.sideboard_cards.iter().flat_map(|sbs| {
            sbs.iter().flat_map(|sb| {
                sb.cards_in_sideboard.iter().sorted().dedup_with_count().map(|(count, card)| {
                    (card, count, Zone::Sideboard { sb_card_id: sb.sideboard_card.id })
                })
            })
        }))
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|(card, count, zone)| {
            let slug = draw_card_slug(card, count, zone);
            ((card.id, zone), slug)
        })
        .collect::<HashMap<_, _>>()
}

fn draw_heading_slug(heading: &str) -> RgbaImage {
    let mut img = draw_main_canvas(SLUG_WIDTH, CROP_HEIGHT, [255; 4]);
    draw_text(&mut img, [10, 10, 10, 255], 15, HEADING_SCALE, heading);
    img
}

fn draw_main_canvas(width: u32, height: u32, color: impl Into<Rgba<u8>>) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    drawing::draw_filled_rect_mut(&mut img, Rect::at(0, 0).of_size(width, height), color.into());
    img
}

fn draw_deck_title(img: &mut RgbaImage, deck: &Deck) -> Result<()> {
    let offset = if let Ok(class_img) = get_class_icon(deck.class) {
        img.copy_from(
            &imageops::resize(&class_img, CROP_HEIGHT, CROP_HEIGHT, imageops::FilterType::Gaussian),
            MARGIN,
            MARGIN,
        )?;
        MARGIN + CROP_HEIGHT + 10
    } else {
        MARGIN
    };

    draw_text(img, [10, 10, 10, 255], offset, HEADING_SCALE, &deck.title);

    Ok(())
}

#[cached::proc_macro::cached(result = true)]
fn get_class_icon(class: Class) -> Result<RgbaImage> {
    if class == Class::Neutral {
        anyhow::bail!("No neutral class icon");
    }

    let link = format!(
        "https://render.worldofwarcraft.com/us/icons/56/classicon_{}.jpg",
        class.in_en_us().to_string().to_ascii_lowercase().replace(' ', "")
    );

    let mut buf = Vec::new();
    AGENT.get(&link).call()?.into_reader().read_to_end(&mut buf)?;

    Ok(image::load_from_memory(&buf)?.into())
}

#[cached::proc_macro::cached(
    time = 86400, // one day.
    time_refresh = true,
    result = true,
    key = "usize",
    convert = r#"{(card.id)}"#
)]
fn get_crop_image(card: &Card) -> Result<RgbaImage> {
    let link = card
        .crop_image
        .clone()
        .or_else(|| crate::card_details::get_hearth_sim_crop_image(card.id))
        .unwrap_or("https://art.hearthstonejson.com/v1/tiles/GAME_006.png".into());

    let mut buf = Vec::new();
    AGENT.get(&link).call()?.into_reader().read_to_end(&mut buf)?;

    Ok(image::load_from_memory(&buf)?.into())
}

fn draw_text<'a>(
    canvas: &'a mut RgbaImage,
    color: impl Into<Rgba<u8>> + Copy,
    x_offset: u32,
    scale: f32,
    text: &'a str,
) {
    let (image_width, image_height) = canvas.dimensions();

    let mut caret = 0.0;
    let v_metric = FONTS[0].0.as_scaled(scale).ascent();
    let y_offset = {
        let mut y = (CROP_HEIGHT - v_metric as u32) / 2;

        // band-aid for Deck title:
        if image_height > CROP_HEIGHT {
            y += MARGIN;
        }
        y
    };

    for c in text.chars() {
        let Some((f_f, f_s)) = FONTS.iter().find(|(f_f, _)| f_f.glyph_id(c).0 > 0) else {
            continue;
        };

        let f_f = f_f.as_scaled(scale * f_s);

        let mut g = f_f.scaled_glyph(c);
        g.position = (caret, v_metric).into();

        caret += f_f.h_advance(g.id);

        let Some(g) = f_f.outline_glyph(g) else { continue };

        let bb = g.px_bounds();
        g.draw(|gx, gy, gv| {
            let image_x = gx + bb.min.x as u32 + x_offset;
            let image_y = gy + bb.min.y as u32 + y_offset;

            if (0..image_width).contains(&image_x) && (0..image_height).contains(&image_y) {
                let pixel = canvas.get_pixel(image_x, image_y);
                let weighted_color = interpolate(color.into(), *pixel, gv);
                canvas.draw_pixel(image_x, image_y, weighted_color);
            }
        });
    }
}
