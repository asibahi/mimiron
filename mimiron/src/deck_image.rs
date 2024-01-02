#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::cast_sign_loss
)]

use crate::{
    card::Card,
    card_details::{Class, Locale, Localize, Rarity},
    deck::Deck,
    helpers::{get_boxes_and_glue, TextStyle},
    AGENT,
};
use anyhow::{anyhow, Result};
use image::{imageops, DynamicImage, GenericImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::{
    drawing::{self, Canvas as _},
    pixelops::weighted_sum,
    rect::Rect,
};
use rayon::prelude::*;
use rusttype::{Font, Scale};
use std::collections::{BTreeMap, HashMap};

//  Numbers based on the crops provided by Blizzard API
const CROP_WIDTH: u32 = 243;
const CROP_HEIGHT: u32 = 64;

const MARGIN: u32 = 5;

const SLUG_WIDTH: u32 = CROP_WIDTH * 2 + CROP_HEIGHT;
const ROW_HEIGHT: u32 = CROP_HEIGHT + MARGIN;
const COLUMN_WIDTH: u32 = SLUG_WIDTH + MARGIN;

const TEXT_BOX_HEIGHT: u32 = CROP_HEIGHT; // two lines height + 10 margin. subject to change.
const SLUG_HEIGHT_WITH_TEXT: u32 = CROP_HEIGHT + TEXT_BOX_HEIGHT;
const ROW_HEIGHT_WITH_TEXT: u32 = SLUG_HEIGHT_WITH_TEXT + MARGIN;

const CARD_NAME_FONT: &[u8] =
    include_bytes!("../data/YanoneKaffeesatz/YanoneKaffeesatz-Medium.ttf");

const TEXT_PLAIN_FONT: &[u8] = include_bytes!("../data/Roboto/Roboto-Regular.ttf");
const TEXT_BOLD_FONT: &[u8] = include_bytes!("../data/Roboto/Roboto-Medium.ttf");
const TEXT_ITALIC_FONT: &[u8] = include_bytes!("../data/Roboto/Roboto-Italic.ttf");
const TEXT_BOLD_ITALIC_FONT: &[u8] = include_bytes!("../data/Roboto/Roboto-MediumItalic.ttf");

const FALLBACK_PLAIN_FONTS: [&[u8]; 5] = [
    include_bytes!("../data/Noto/Noto_Sans_JP/NotoSansJP-Regular.ttf"),
    include_bytes!("../data/Noto/Noto_Sans_KR/NotoSansKR-Regular.ttf"),
    include_bytes!("../data/Noto/Noto_Sans_SC/NotoSansSC-Regular.ttf"),
    include_bytes!("../data/Noto/Noto_Sans_TC/NotoSansTC-Regular.ttf"),
    include_bytes!("../data/Noto/Noto_Sans_Thai_Looped/NotoSansThaiLooped-Regular.ttf"),
];

const FALLBACK_BOLD_FONTS: [&[u8]; 5] = [
    include_bytes!("../data/Noto/Noto_Sans_JP/NotoSansJP-Medium.ttf"),
    include_bytes!("../data/Noto/Noto_Sans_KR/NotoSansKR-Medium.ttf"),
    include_bytes!("../data/Noto/Noto_Sans_SC/NotoSansSC-Medium.ttf"),
    include_bytes!("../data/Noto/Noto_Sans_TC/NotoSansTC-Medium.ttf"),
    include_bytes!("../data/Noto/Noto_Sans_Thai_Looped/NotoSansThaiLooped-Medium.ttf"),
];

const FALLBACK_LIGHT_FONTS: [&[u8]; 5] = [
    // italic not available for these fonts.
    include_bytes!("../data/Noto/Noto_Sans_JP/NotoSansJP-Light.ttf"),
    include_bytes!("../data/Noto/Noto_Sans_KR/NotoSansKR-Light.ttf"),
    include_bytes!("../data/Noto/Noto_Sans_SC/NotoSansSC-Light.ttf"),
    include_bytes!("../data/Noto/Noto_Sans_TC/NotoSansTC-Light.ttf"),
    include_bytes!("../data/Noto/Noto_Sans_Thai_Looped/NotoSansThaiLooped-Light.ttf"),
];

#[derive(Clone, Copy)]
pub enum ImageOptions {
    /// Each group in its own column. (HS Top Decks)
    Groups,

    Regular {
        /// 1 is most compact horizontally.
        /// 3 is most compact (yet readable) vertically.
        columns: u8,

        /// whether card text is included. Best with 3 columns.
        /// Currently broken for non-Latin alphabet locales.
        with_text: bool,
    },

    /// Similar to Regular but is either 2 or 3 columns based on "size".
    Adaptable,
}

pub fn get(deck: &Deck, locale: Locale, shape: ImageOptions) -> Result<DynamicImage> {
    match shape {
        ImageOptions::Groups => img_groups_format(deck, locale),
        ImageOptions::Regular { columns, with_text } => {
            img_columns_format(deck, locale, columns as u32, with_text)
        }
        ImageOptions::Adaptable => img_adaptable_format(deck, locale),
    }
}

fn img_adaptable_format(deck: &Deck, locale: Locale) -> Result<DynamicImage> {
    let (ordered_cards, slug_map) = order_deck_and_get_slugs(deck, locale, false);

    let (mut img, cards_in_col) = {
        let main_deck_length = ordered_cards.len();

        let sideboards_length = deck.sideboard_cards.as_ref().map_or(0, |sbs| {
            sbs.iter()
                .fold(0, |acc, sb| sb.cards_in_sideboard.len() + 1 + acc)
        });

        let length = (main_deck_length + sideboards_length) as u32;

        // slightly more sophisticated hack for Reno Renathal decks.
        let col_count = (length / 15 + 1).max(2);
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
                &get_heading_slug(&format!("Sideboard: {}", sb.sideboard_card.name)),
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

fn img_columns_format(
    deck: &Deck,
    locale: Locale,
    col_count: u32,
    with_text: bool,
) -> Result<DynamicImage> {
    let (ordered_cards, slug_map) = order_deck_and_get_slugs(deck, locale, with_text);

    let actual_row_height = if with_text {
        ROW_HEIGHT_WITH_TEXT
    } else {
        ROW_HEIGHT
    };

    let (mut img, cards_in_col) = {
        let main_deck_length = ordered_cards.len();

        let sideboards_length = deck.sideboard_cards.as_ref().map_or(0, |sbs| {
            sbs.iter()
                .fold(0, |acc, sb| sb.cards_in_sideboard.len() + 1 + acc)
        });

        let length = (main_deck_length + sideboards_length) as u32;

        let cards_in_col = length / col_count + (length % col_count).min(1);

        // main canvas
        let img = draw_main_canvas(
            COLUMN_WIDTH * col_count + MARGIN,
            ROW_HEIGHT + cards_in_col * actual_row_height + MARGIN,
            (255, 255, 255),
        );

        (img, cards_in_col)
    };

    draw_deck_title(&mut img, locale, deck)?;

    // Main deck
    for (i, (card, _)) in ordered_cards.iter().enumerate() {
        let slug = &slug_map[card];

        let i = i as u32;
        let (col, row) = (i / cards_in_col, i % cards_in_col);

        img.copy_from(
            slug,
            col * COLUMN_WIDTH + MARGIN,
            ROW_HEIGHT + row * actual_row_height + MARGIN,
        )?;
    }

    // sideboard cards
    if let Some(sideboards) = &deck.sideboard_cards {
        let mut sb_pos_tracker = ordered_cards.len() as u32;

        for sb in sideboards {
            let (col, row) = (sb_pos_tracker / cards_in_col, sb_pos_tracker % cards_in_col);
            let title_offset = if with_text { TEXT_BOX_HEIGHT } else { 0 };

            img.copy_from(
                &get_heading_slug(&format!("Sideboard: {}", sb.sideboard_card.name)),
                col * COLUMN_WIDTH + MARGIN,
                ROW_HEIGHT + title_offset + row * actual_row_height + MARGIN,
            )?;
            sb_pos_tracker += 1;

            for slug in order_cards(&sb.cards_in_sideboard)
                .keys()
                .map(|c| &slug_map[c])
            {
                let i = sb_pos_tracker;
                let (col, row) = (i / cards_in_col, i % cards_in_col);
                img.copy_from(
                    slug,
                    col * COLUMN_WIDTH + MARGIN,
                    ROW_HEIGHT + row * actual_row_height + MARGIN,
                )?;

                sb_pos_tracker += 1;
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(img))
}

fn img_groups_format(deck: &Deck, locale: Locale) -> Result<DynamicImage> {
    let (ordered_cards, slug_map) = order_deck_and_get_slugs(deck, locale, false);

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

    draw_deck_title(&mut img, locale, deck)?;

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
                &get_heading_slug(&format!("Sideboard: {}", sb.sideboard_card.name)),
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

fn get_card_slug(card: &Card, locale: Locale, count: usize, with_text: bool) -> DynamicImage {
    assert!(count > 0);

    let name = &card.name;

    let r_color = &card.rarity.color();
    let c_color = card.class.iter().next().unwrap_or(&Class::Neutral).color();

    let slug_height = if with_text {
        SLUG_HEIGHT_WITH_TEXT
    } else {
        CROP_HEIGHT
    };

    // main canvas
    let mut img = draw_main_canvas(SLUG_WIDTH, slug_height, (10, 10, 10));

    if with_text {
        let text_box = build_text_box(
            &(format!("{:#} {}", &card.card_type.in_locale(locale), card.text)),
            c_color,
        );
        img.copy_from(&text_box, 0, CROP_HEIGHT).ok();
    }

    if let Err(e) = draw_crop_image(&mut img, card) {
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

    // font and size
    let font = Font::try_from_bytes(CARD_NAME_FONT).unwrap();
    let fallback_fonts = FALLBACK_PLAIN_FONTS.map(|s| Font::try_from_bytes(s).unwrap());
    let scale = Scale::uniform(40.0);

    // card name
    draw_text(
        &mut img,
        (255, 255, 255),
        CROP_HEIGHT as i32 + 10,
        15,
        scale,
        Scale{ x: 40.0, y: 50.0 },
        &font,
        &fallback_fonts,
        name, //.to_uppercase(),
    );

    // mana square
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(CROP_HEIGHT, CROP_HEIGHT),
        Rgba([60, 109, 173, 255]),
    );

    // card cost
    let cost = card.cost.to_string();
    let (tw, _) = drawing::text_size(scale, &font, &cost);
    draw_text(
        &mut img,
        (255, 255, 255),
        (CROP_HEIGHT as i32 - tw) / 2,
        15,
        scale,
        scale, // pointless field here.
        &font,
        &fallback_fonts, // pointless field here.
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
    let (tw, _) = drawing::text_size(scale, &font, &count);
    draw_text(
        &mut img,
        (255, 255, 255),
        SLUG_WIDTH as i32 - (CROP_HEIGHT as i32 + tw) / 2,
        15,
        scale,
        scale, // pointless field here
        &font,
        &fallback_fonts, // pointes field here.
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

fn order_deck_and_get_slugs(
    deck: &Deck,
    locale: Locale,
    with_text: bool,
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

    // if a card is in two zones it'd have the same slug in both.
    let slug_map = ordered_cards
        .clone()
        .into_par_iter()
        .chain(ordered_sbs_cards.into_par_iter())
        .map(|(card, count)| {
            let slug = get_card_slug(card, locale, count, with_text);
            (card, slug)
        })
        .collect::<HashMap<_, _>>();

    (ordered_cards, slug_map)
}

fn get_heading_slug(heading: &str) -> DynamicImage {
    // main canvas
    let mut img = draw_main_canvas(SLUG_WIDTH, CROP_HEIGHT, (255, 255, 255));

    // font and size
    let font = Font::try_from_bytes(CARD_NAME_FONT).unwrap();
    let fallback_fonts = FALLBACK_PLAIN_FONTS.map(|s| Font::try_from_bytes(s).unwrap());
    let scale = Scale::uniform(50.0);

    let (_, th) = drawing::text_size(scale, &font, "E");

    draw_text(
        &mut img,
        (10, 10, 10),
        15,
        (CROP_HEIGHT as i32 - th) / 2,
        scale,
        Scale::uniform(60.0),
        &font,
        &fallback_fonts,
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
        format!(
            "{} - {}",
            deck.class.in_locale(locale),
            deck.format.to_uppercase()
        )
    });

    // font and size
    let font = Font::try_from_bytes(CARD_NAME_FONT).unwrap();
    let fallback_fonts = FALLBACK_PLAIN_FONTS.map(|s| Font::try_from_bytes(s).unwrap());
    let scale = Scale::uniform(50.0);

    let (_, th) = drawing::text_size(scale, &font, "E");

    // title
    draw_text(
        img,
        (10, 10, 10),
        MARGIN as i32 + CROP_HEIGHT as i32 + 10,
        MARGIN as i32 + (CROP_HEIGHT as i32 - th) / 2,
        scale,
        Scale::uniform(60.0),
        &font,
        &fallback_fonts,
        &title,
    );

    if let Ok(class_img) = get_class_icon(&deck.class) {
        img.copy_from(
            &class_img.resize_to_fill(CROP_HEIGHT, CROP_HEIGHT, imageops::FilterType::Gaussian),
            MARGIN,
            MARGIN,
        )?;
    }

    Ok(())
}

fn get_class_icon(class: &Class) -> Result<DynamicImage> {
    let mut buf = Vec::new();
    AGENT
        .get(
            &(format!(
                "https://render.worldofwarcraft.com/us/icons/56/classicon_{}.jpg",
                class.in_locale(Locale::enUS).to_string().to_lowercase()
            )),
        )
        .call()?
        .into_reader()
        .read_to_end(&mut buf)?;

    Ok(image::load_from_memory(&buf)?)
}

fn draw_crop_image(img: &mut RgbaImage, card: &Card) -> Result<()> {
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
    AGENT
        .get(&link)
        .call()?
        .into_reader()
        .read_to_end(&mut buf)?;

    let crop = image::load_from_memory(&buf)?;

    img.copy_from(&crop, CROP_WIDTH, 0)?;

    Ok(())
}

fn build_text_box(text: &str, color: (u8, u8, u8)) -> DynamicImage {
    let plain_font = Font::try_from_bytes(TEXT_PLAIN_FONT).unwrap();
    let fallback_plains = FALLBACK_PLAIN_FONTS.map(|s| Font::try_from_bytes(s).unwrap());

    let bold_font = Font::try_from_bytes(TEXT_BOLD_FONT).unwrap();
    let fallback_bolds = FALLBACK_BOLD_FONTS.map(|s| Font::try_from_bytes(s).unwrap());

    let italic_font = Font::try_from_bytes(TEXT_ITALIC_FONT).unwrap();
    let fallback_lights = FALLBACK_LIGHT_FONTS.map(|s| Font::try_from_bytes(s).unwrap());

    let bold_italic_font = Font::try_from_bytes(TEXT_BOLD_ITALIC_FONT).unwrap();
    // for fallback, use plains. Bold of Light! Noto Italics not available for some reason.

    // main canvas.
    let mut img = ImageBuffer::new(SLUG_WIDTH, TEXT_BOX_HEIGHT);
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(SLUG_WIDTH, TEXT_BOX_HEIGHT),
        Rgba([10, 10, 10, 255]),
    );

    // class color
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(CROP_WIDTH as i32, 0).of_size(CROP_WIDTH + CROP_HEIGHT, TEXT_BOX_HEIGHT),
        Rgba([color.0, color.1, color.2, 170]),
    );

    // gradient
    let mut gradient = RgbaImage::new(CROP_WIDTH, TEXT_BOX_HEIGHT);
    imageops::horizontal_gradient(
        &mut gradient,
        &Rgba([10, 10, 10, 255]),
        &Rgba([10, 10, 10, 0]),
    );

    // img.copy_from(&gradient, CROP_WIDTH, 0).ok();
    imageops::overlay(&mut img, &gradient, CROP_WIDTH as i64, 0);

    let scale = Scale::uniform(20.0);
    let space_advance = plain_font
        .glyph(' ')
        .scaled(scale)
        .h_metrics()
        .advance_width as i32;

    let line_height = drawing::text_size(scale, &plain_font, "O").1 * 14 / 10;

    let mut cursor = (15, 10);

    for bx in get_boxes_and_glue(text) {
        let font = match bx.style() {
            TextStyle::Plain => &plain_font,
            TextStyle::Bold => &bold_font,
            TextStyle::Italic => &italic_font,
            TextStyle::BoldItalic => &bold_italic_font,
        };

        let fallback_fonts = match bx.style() {
            TextStyle::Plain | TextStyle::BoldItalic => &fallback_plains,
            TextStyle::Bold => &fallback_bolds,
            TextStyle::Italic => &fallback_lights,
        };

        // Breaks text wrapping completely for non-Latin scripts.
        // will only bother fixing if I see it used.
        let box_advance = drawing::text_size(scale, font, &bx.text()).0;

        if SLUG_WIDTH as i32 - 20 <= cursor.0 + box_advance {
            cursor = (15, cursor.1 + line_height);
        }

        draw_text(
            &mut img,
            (255, 255, 255),
            cursor.0,
            cursor.1,
            scale,
            scale, // switch later from Roboto to Noto for Latin
            font,
            fallback_fonts,
            &bx.text(),
        );

        cursor.0 += box_advance // imageproc trims end spaces.
            + if bx.text().chars().last().is_some_and(char::is_whitespace) {
                space_advance
            } else {
                0
            };
    }

    DynamicImage::ImageRgba8(img)
}

// isolate the function to inline `imageproc::drawing::draw_text_mut` and impl font fallback.
fn draw_text<'a>(
    canvas: &'a mut RgbaImage,
    color: (u8, u8, u8),
    x: i32,
    y: i32,
    scale: Scale,
    fallback_scale: Scale, // Noto fonts smaller than Yanone.
    font: &'a Font<'a>,
    fallback_fonts: &'a [Font<'a>],
    text: &'a str,
) {
    let image_width = canvas.width() as i32;
    let image_height = canvas.height() as i32;

    // let mut last_glyph = None; // kerning tool

    let mut caret = 0.0;
    let v_metric = font.v_metrics(scale).ascent;

    'layout: for c in text.chars() {
        let mut g = font.glyph(c).scaled(scale);

        'fallback: {
            if g.id().0 == 0 {
                // glyph not in the font files
                for fallback_font in fallback_fonts {
                    let inner_g = fallback_font.glyph(c).scaled(fallback_scale);
                    if inner_g.id().0 > 0 {
                        g = inner_g;
                        break 'fallback;
                    }
                }
                continue 'layout;
            }
        }

        // if let Some(last) = last_glyph {
        // caret += font.pair_kerning(scale, last, g.id()); // kerning tool
        // }

        let g = g.positioned(rusttype::point(caret, v_metric));

        caret += g.unpositioned().h_metrics().advance_width;

        // last_glyph = Some(g.id()); // kerning tol

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
