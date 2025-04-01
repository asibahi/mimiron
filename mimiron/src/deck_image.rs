#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::cast_sign_loss
)]

use crate::{
    AGENT,
    card::Card,
    card_details::{CardType, Class, Rarity},
    deck::Deck,
    hearth_sim::{get_hearth_sim_crop_image, get_hearth_sim_details},
    localization::Localize,
};
use ab_glyph::{Font, FontRef, ScaleFont};
use anyhow::Result;
use compact_str::{CompactString, ToCompactString, format_compact};
use image::{GenericImage, GenericImageView, Rgba, RgbaImage, imageops};
use imageproc::{drawing, pixelops::interpolate, rect::Rect};
use itertools::Itertools;
use rayon::prelude::*;
use std::{collections::HashMap, num::NonZeroU32, ops::Not, sync::LazyLock};

// Numbers based on the crops provided by Blizzard API
const CROP_WIDTH        : u32 = 243;
const CROP_HEIGHT       : u32 = 64;

const INFO_WIDTH        : u32 = CROP_HEIGHT;
const COLOR_BAND_WIDTH  : u32 = CROP_HEIGHT / 8;
const MANA_WIDTH        : u32 = INFO_WIDTH - COLOR_BAND_WIDTH;

const MARGIN            : u32 = 5;

const SLUG_WIDTH        : u32 = CROP_WIDTH * 2 + INFO_WIDTH;
const ROW_HEIGHT        : u32 = CROP_HEIGHT + MARGIN;
const COLUMN_WIDTH      : u32 = SLUG_WIDTH + MARGIN;

const CROP_IMAGE_OFFSET : u32 = SLUG_WIDTH - CROP_WIDTH - INFO_WIDTH;

const HEADING_SCALE     : f32 = 50.0;
const CARD_NAME_SCALE   : f32 = 40.0;

macro_rules! lazy {
    ($s:literal, $f: literal) => {
        (LazyLock::new(|| FontRef::try_from_slice(include_bytes!(concat!("../fonts/", $s))).unwrap()), $f)
    };
}

// potential here to cut memory usage of the bot.
static FONTS: [(LazyLock<FontRef<'_>>, f32); 4] = [
    // Base font
    lazy!("YanoneKaffeesatz-Medium.ttf", 1.0),
    
    // Fallbacks
    lazy!("NotoSansCJK-Medium.ttc", 1.2),
    lazy!("NotoSansThaiLooped-Medium.ttf", 1.3),

    // pixel font
    lazy!("Jersey10-Regular.ttf", 1.0),
];

#[derive(Clone, Copy)]
pub enum ImageOptions {
    /// Each group in its own column. (HS Top Decks)
    Groups,

    Regular {
        /// 1 is most compact horizontally.
        /// 3 is most compact (yet readable) vertically.
        columns: u8,

        /// Whether sideboards are inline, or at the end of Deck
        inline_sideboard: bool,
    },

    /// Similar to Regular but is either 2 or 3 columns based on "size".
    /// Sideboards are inlined
    Adaptable,
}

pub fn get(deck: &Deck, shape: ImageOptions) -> RgbaImage {
    match shape {
        ImageOptions::Groups => img_groups_format(deck),
        ImageOptions::Adaptable => img_columns_format(deck, None, true),
        ImageOptions::Regular { columns, inline_sideboard } =>
            img_columns_format(deck, NonZeroU32::new(columns as u32), inline_sideboard),
    }
}

fn img_columns_format(
    deck: &Deck,
    col_count: Option<NonZeroU32>,
    inline_sideboard: bool,
) -> RgbaImage {
    let ordered_main_deck = deck.cards.iter().sorted().dedup();
    let slug_map = get_cards_slugs(
        deck,
        if inline_sideboard { SideboardStyle::Indented } else { SideboardStyle::EndOfDeck },
    );

    let (mut img, pos_in_img) = {
        let length = (slug_map.len()
            + deck.sideboard_cards
                .as_ref()
                .filter(|_| !inline_sideboard)
                .map_or(0, Vec::len)) as u32;

        let col_count =
            col_count.map_or_else(|| (length / 15 + (length % 15).min(1)).max(2), u32::from);
        let cards_in_col = length / col_count + (length % col_count).min(1);

        let vertical_title = col_count == 1;

        let mut img = if vertical_title {
            RgbaImage::from_pixel(
                ROW_HEIGHT * cards_in_col + 4 * MARGIN,
                COLUMN_WIDTH + ROW_HEIGHT + MARGIN,
                Rgba([255; 4]),
            )
        } else {
            RgbaImage::from_pixel(
                COLUMN_WIDTH * col_count + MARGIN,
                ROW_HEIGHT * (cards_in_col + 1) + 4 * MARGIN,
                Rgba([255; 4]),
            )
        };

        draw_deck_title(&mut img, deck, vertical_title);
        if vertical_title {
            img = imageops::rotate90(&img);
        }

        draw_footer(&mut img, deck.class.color());

        (img, move |c| (c / cards_in_col, c % cards_in_col + (!vertical_title) as u32))
    };

    let mut cursor = 0;

    for card in ordered_main_deck {
        let slug = &slug_map[&(card.id, Zone::MainDeck)];

        let (col, row) = pos_in_img(cursor);

        _ = img.copy_from(slug, col * COLUMN_WIDTH + MARGIN, row * ROW_HEIGHT + MARGIN);

        cursor += 1;

        if inline_sideboard {
            for slug in deck
                .sideboard_cards
                .iter()
                .flatten()
                .filter(|sb| sb.sideboard_card.id == card.id)
                .flat_map(|sb| sb.cards_in_sideboard.iter().sorted().dedup())
                .map(|c| &slug_map[&(c.id, Zone::Sideboard { sb_card_id: card.id })])
            {
                let (col, row) = pos_in_img(cursor);

                _ = img.copy_from(slug, col * COLUMN_WIDTH + MARGIN, row * ROW_HEIGHT + MARGIN);
                cursor += 1;
            }
        }
    }

    if inline_sideboard.not() {
        for sb in deck.sideboard_cards.iter().flatten() {
            let (col, row) = pos_in_img(cursor);
            _ = img.copy_from(
                &draw_heading_slug(&format_compact!("> {}", sb.sideboard_card.name)),
                col * COLUMN_WIDTH + MARGIN,
                row * ROW_HEIGHT + MARGIN,
            );
            cursor += 1;

            for slug in
                sb.cards_in_sideboard.iter().sorted().dedup().map(|c|
                    &slug_map[&(c.id, Zone::Sideboard { sb_card_id: sb.sideboard_card.id })]
                )
            {
                let (col, row) = pos_in_img(cursor);
                _ = img.copy_from(slug, col * COLUMN_WIDTH + MARGIN, row * ROW_HEIGHT + MARGIN);

                cursor += 1;
            }
        }
    }

    img
}

fn img_groups_format(deck: &Deck) -> RgbaImage {
    let ordered_main_deck = deck.cards.iter().sorted().dedup();
    let slug_map = get_cards_slugs(deck, SideboardStyle::EndOfDeck);

    let class_cards = ordered_main_deck
        .clone()
        .filter(|&c| c.class.is_empty().not())
        .map(|c| &slug_map[&(c.id, Zone::MainDeck)])
        .enumerate()
        .collect::<Vec<_>>();

    let neutral_cards = ordered_main_deck
        .filter(|&c| c.class.is_empty())
        .map(|c| &slug_map[&(c.id, Zone::MainDeck)])
        .enumerate()
        .collect::<Vec<_>>();

    let mut img = {
        // assumes decks will always have class cards
        let mut columns = 1;
        if neutral_cards.is_empty().not() {
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

        RgbaImage::from_pixel(
            columns * COLUMN_WIDTH + MARGIN,
            rows * ROW_HEIGHT + 4 * MARGIN,
            Rgba([255; 4]),
        )
    };

    draw_deck_title(&mut img, deck, false);
    draw_footer(&mut img, deck.class.color());

    for (i, slug) in class_cards {
        let i = i as u32 + 1;
        _ = img.copy_from(slug, MARGIN, i * ROW_HEIGHT + MARGIN);
    }

    for (i, slug) in neutral_cards {
        let i = i as u32 + 1;
        _ = img.copy_from(slug, COLUMN_WIDTH + MARGIN, i * ROW_HEIGHT + MARGIN);
    }

    if let Some(sideboards) = &deck.sideboard_cards {
        // always last column
        let sb_col = img.width() - COLUMN_WIDTH;
        let mut sb_cursor = 1;

        for sb in sideboards {
            _ = img.copy_from(
                &draw_heading_slug(&format_compact!("> {}", sb.sideboard_card.name)),
                sb_col,
                sb_cursor * ROW_HEIGHT + MARGIN,
            );
            sb_cursor += 1;

            for slug in
                sb.cards_in_sideboard.iter().sorted().dedup().map(|c|
                    &slug_map[&(c.id, Zone::Sideboard { sb_card_id: sb.sideboard_card.id })]
                )
            {
                _ = img.copy_from(slug, sb_col, sb_cursor * ROW_HEIGHT + MARGIN);
                sb_cursor += 1;
            }
        }
    }

    img
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
enum Zone {
    MainDeck,
    Sideboard { sb_card_id: usize },
}

#[derive(Clone, Copy)]
enum SideboardStyle { EndOfDeck, Indented }

fn draw_card_slug(card: &Card, count: usize, zone: Zone, sb_style: SideboardStyle) -> RgbaImage {
    assert!(count > 0);

    // if card type is Unknown data other than card id is usually junk.
    let (name, cost, rarity) = matches!(card.card_type, CardType::Unknown)
        .then(|| get_hearth_sim_details(card.id))
        .flatten()
        .unwrap_or_else(|| (card.name.clone(), card.cost, card.rarity));

    let alpha = |(x, y, z)| [x, y, z, 255];

    let r_color = alpha(rarity.color());
    let c_color = card.class.iter().map(|c| alpha(c.color())).collect::<Vec<_>>();

    let indent = match (zone, sb_style) {
        (Zone::MainDeck, _) | (_, SideboardStyle::EndOfDeck) => 0,
        (Zone::Sideboard { .. }, SideboardStyle::Indented) => INFO_WIDTH / 3,
    };

    // main canvas
    let mut img = RgbaImage::from_fn(SLUG_WIDTH, CROP_HEIGHT, |x, y|
        match x {
            // Legendary color for Sideboard indent
            _ if x < indent.saturating_sub(MARGIN) => alpha(Rarity::Legendary.color()),

            // gap between Sideboard marker and Mana Square
            _ if x < indent => [255; 4],

            // Mana Square
            _ if x <= indent + MANA_WIDTH => [54, 98, 156, 255],

            // Class color band
            _ if x <= indent + MANA_WIDTH + COLOR_BAND_WIDTH => {
                let idx = y * c_color.len() as u32 / CROP_HEIGHT;
                // Neutral color
                c_color.get(idx as usize).copied().unwrap_or([169, 169, 169, 255])
            }
            _ => [10, 10, 10, 255],
        }
        .into()
    );

    match get_crop_image(card).and_then(|crop| Ok(img.copy_from(&crop, CROP_IMAGE_OFFSET, 0)?)) {
        Ok(()) => {
            let mut gradient = RgbaImage::new(CROP_WIDTH, CROP_HEIGHT);
            imageops::horizontal_gradient(
                &mut gradient,
                &Rgba([10u8, 10, 10, 255]),
                &Rgba([10u8, 10, 10, 0]),
            );
            imageops::overlay(&mut img, &gradient, CROP_IMAGE_OFFSET as i64, 0);
        }
        Err(e) => {
            tracing::warn!("Failed to get image of {name}: {e}.");
            imageops::horizontal_gradient(
                &mut *imageops::crop(&mut img, CROP_IMAGE_OFFSET, 0, CROP_WIDTH, CROP_HEIGHT),
                &Rgba([10u8, 10, 10, 255]),
                &Rgba(r_color),
            );
        }
    }

    // card name
    draw_text(&mut img, [255; 4], indent + INFO_WIDTH + 10, 0, CARD_NAME_SCALE, &name);

    // card cost
    let cost = cost.to_compact_string();
    let (tw, _) = drawing::text_size(CARD_NAME_SCALE, &*FONTS[0].0, &cost);
    draw_text(
        &mut img,
        [255; 4],
        indent + (MANA_WIDTH.saturating_sub(tw)) / 2,
        0,
        CARD_NAME_SCALE,
        &cost,
    );

    // rarity square
    // drawn latest to overlap previous elements.
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at((SLUG_WIDTH - INFO_WIDTH) as i32, 0).of_size(INFO_WIDTH, CROP_HEIGHT),
        Rgba(r_color),
    );

    // card count
    let count = match (count, rarity) {
        (1, Rarity::Noncollectible) => CompactString::from("!"),
        (1, Rarity::Legendary) => CompactString::default(),
        _ => count.to_compact_string(),
    };
    let (tw, _) = drawing::text_size(CARD_NAME_SCALE, &*FONTS[0].0, &count);
    draw_text(&mut img, [255; 4], SLUG_WIDTH - (INFO_WIDTH + tw) / 2, 0, CARD_NAME_SCALE, &count);

    img
}

fn get_cards_slugs(deck: &Deck, sb_style: SideboardStyle) -> HashMap<(usize, Zone), RgbaImage> {
    deck.cards
        .iter()
        .sorted()
        .dedup_with_count()
        .map(|(count, card)| (card, count, Zone::MainDeck))
        .chain(deck.sideboard_cards.iter().flat_map(
            |sbs| sbs.iter().flat_map(
                |sb| sb.cards_in_sideboard.iter().sorted().dedup_with_count().map(
                    |(count, card)| (card, count, Zone::Sideboard { sb_card_id: sb.sideboard_card.id })
                )
            )
        ))
        .par_bridge()
        .map(|(card, count, zone)| {
            let slug = draw_card_slug(card, count, zone, sb_style);
            ((card.id, zone), slug)
        })
        .collect()
}

fn draw_heading_slug(heading: &str) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(SLUG_WIDTH, CROP_HEIGHT, Rgba([255; 4]));
    draw_text(&mut img, [10, 10, 10, 255], 15, 0, HEADING_SCALE, heading);
    img
}

fn draw_deck_title(img: &mut RgbaImage, deck: &Deck, vertical: bool) {
    let offset = get_class_icon(deck.class).map_or(MARGIN, |class_img| {
        let mut class_img =
            imageops::resize(&class_img, INFO_WIDTH, CROP_HEIGHT, imageops::FilterType::Gaussian);
        if vertical {
            class_img = imageops::rotate270(&class_img);
        }
        img.copy_from(&class_img, MARGIN, MARGIN)
            .expect("class thumbnail can't be larger than image!!");
        MARGIN + INFO_WIDTH + 10
    });

    draw_text(img, [10, 10, 10, 255], offset, MARGIN, HEADING_SCALE, &deck.title);
}

fn draw_footer(img: &mut RgbaImage, (r, g, b): (u8, u8, u8)) {
    let text = "github.com/asibahi/mimiron";
    let (tw, th) = drawing::text_size(20.0, &*FONTS[3].0, text);

    let h_offset = (img.width() - (tw + MARGIN)) as i32;
    let v_offset = (img.height() - (th + 2 * MARGIN)) as i32;

    drawing::draw_text_mut(
        img,
        Rgba([10, 10, 10, 255]),
        h_offset,
        v_offset,
        20.0,
        &*FONTS[3].0,
        text,
    );

    drawing::draw_filled_rect_mut(
        img,
        Rect::at(MARGIN as i32, (img.height() - 3 * MARGIN) as i32)
            .of_size(img.width() - (3 * MARGIN + tw), 2 * MARGIN),
        Rgba([r, g, b, 255]),
    );
}

#[cached::proc_macro::cached(result = true)]
fn get_class_icon(class: Class) -> Result<RgbaImage> {
    let link = format!(
        "https://render.worldofwarcraft.com/us/icons/56/classicon_{}.jpg",
        class.in_en_us().to_compact_string().to_ascii_lowercase().replace(' ', "")
    );

    let buf = AGENT.get(link).call()?.body_mut().read_to_vec()?;

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
        .or_else(|| get_hearth_sim_crop_image(card.id))
        .unwrap_or_else(|| "https://art.hearthstonejson.com/v1/tiles/GAME_006.png".into());

    // Might fail but meh. just a crop image.
    let mut counter = 2;
    let buf = loop {
        match AGENT.get(link.as_str()).call() {
            Ok(mut res) => break res.body_mut().read_to_vec()?,
            Err(ureq::Error::Io(err))
                if counter > 0 && err.kind() == std::io::ErrorKind::ConnectionReset =>
            {   // probably not a good idea
                std::thread::sleep(std::time::Duration::from_millis(500));
                counter -= 1;
                continue;
            },
            err => err?,
        };
    };
    Ok(image::load_from_memory(&buf)?.into())
}

fn draw_text(
    canvas: &mut RgbaImage,
    color: impl Into<Rgba<u8>> + Copy,
    x_offset: u32,
    y_offset: u32, // band-aid for Deck Title.
    scale: f32,
    text: &str,
) {
    let mut caret = 0.0;
    let v_metric = FONTS[0].0.as_scaled(scale).ascent();
    let y_offset = (CROP_HEIGHT - v_metric as u32) / 2 + y_offset;

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

            if canvas.in_bounds(image_x, image_y) {
                let pixel = canvas.get_pixel(image_x, image_y);
                let weighted_color = interpolate(color.into(), *pixel, gv);
                canvas.put_pixel(image_x, image_y, weighted_color);
            }
        });
    }
}
