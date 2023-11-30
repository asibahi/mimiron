use crate::{
    card::Card,
    card_details::{Class, Rarity},
    deck::Deck,
    get_agent,
    helpers::{get_boxes_and_glue, TextStyle, Thusable},
};
use anyhow::{anyhow, Result};
use image::{imageops, DynamicImage, GenericImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::{drawing, rect::Rect};
use rayon::prelude::*;
use rusttype::{Font, Scale};
use std::{
    collections::{BTreeMap, HashMap},
    ops::Not,
};

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

pub enum ImageOptions {
    /// Each group in its own column. (HS Top Decks)
    Groups,

    Regular {
        /// 1 is most compact horizontally.
        /// 3 is most compact (yet readable) vertically.
        columns: u8,

        /// whether card text is included. Best with 3 columns.
        with_text: bool,
    },

    /// Similar to Regular but is either 2 or 3 columns based on "size"
    Adaptable,
}

pub fn get(deck: &Deck, shape: ImageOptions) -> Result<DynamicImage> {
    match shape {
        ImageOptions::Groups => img_groups_format(deck),
        ImageOptions::Regular { columns, with_text } => {
            img_columns_format(deck, columns as u32, with_text)
        }
        ImageOptions::Adaptable => img_adaptable_format(deck),
    }
}

fn img_adaptable_format(deck: &Deck) -> Result<DynamicImage> {
    let (ordered_cards, slug_map) = order_deck_and_get_slugs(deck, false);

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

    draw_deck_title(&mut img, deck)?;

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
                &get_title_slug(&format!("Sideboard: {}", sb.sideboard_card.name), 0),
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

fn img_columns_format(deck: &Deck, col_count: u32, with_text: bool) -> Result<DynamicImage> {
    let (ordered_cards, slug_map) = order_deck_and_get_slugs(deck, with_text);

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

    draw_deck_title(&mut img, deck)?;

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
                &get_title_slug(&format!("Sideboard: {}", sb.sideboard_card.name), 0),
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

fn img_groups_format(deck: &Deck) -> Result<DynamicImage> {
    let (ordered_cards, slug_map) = order_deck_and_get_slugs(deck, false);

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
        if neutral_cards.is_empty().not() {
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

    draw_deck_title(&mut img, deck)?;
    if neutral_cards.is_empty().not() {
        let neutrals_title = get_title_slug("Neutrals", 0);
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
            let column_start = COLUMN_WIDTH * (2 + sb_i as u32) + MARGIN;

            img.copy_from(
                &get_title_slug(&format!("Sideboard: {}", sb.sideboard_card.name), 0),
                column_start,
                MARGIN,
            )?;

            for (i, slug) in order_cards(&sb.cards_in_sideboard)
                .iter()
                .enumerate()
                .map(|(i, (c, _))| (i, &slug_map[c]))
            {
                let i = i as u32 + 1;
                img.copy_from(slug, column_start, i * ROW_HEIGHT + MARGIN)?;
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(img))
}

fn get_card_slug(card: &Card, count: usize, with_text: bool) -> DynamicImage {
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
        let text_box = build_text_box(&(format!("{:#} {}", &card.card_type, card.text)), c_color);
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

    // rarity square
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(SLUG_WIDTH as i32 - CROP_HEIGHT as i32, 0).of_size(CROP_HEIGHT, CROP_HEIGHT),
        Rgba([r_color.0, r_color.1, r_color.2, 255]),
    );

    // mana square
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(CROP_HEIGHT, CROP_HEIGHT),
        Rgba([60, 109, 173, 255]),
    );

    // font and size
    let font = Font::try_from_bytes(CARD_NAME_FONT).unwrap();
    let scale = Scale::uniform(40.0);

    // card name
    drawing::draw_text_mut(
        &mut img,
        Rgba([255, 255, 255, 255]),
        CROP_HEIGHT as i32 + 10,
        15,
        scale,
        &font,
        name, //.to_uppercase(),
    );

    // card cost
    let cost = card.cost.to_string();
    let (tw, _) = drawing::text_size(scale, &font, &cost);
    drawing::draw_text_mut(
        &mut img,
        Rgba([255, 255, 255, 255]),
        (CROP_HEIGHT as i32 - tw) / 2,
        15,
        scale,
        &font,
        &cost,
    );

    // card count
    let count = (count > 1 || card.rarity != Rarity::Legendary).thus_or_default(count.to_string());
    let (tw, _) = drawing::text_size(scale, &font, &count);
    drawing::draw_text_mut(
        &mut img,
        Rgba([255, 255, 255, 255]),
        SLUG_WIDTH as i32 - (CROP_HEIGHT as i32 + tw) / 2,
        15,
        scale,
        &font,
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
            let slug = get_card_slug(card, count, with_text);
            (card, slug)
        })
        .collect::<HashMap<_, _>>();

    (ordered_cards, slug_map)
}

fn get_title_slug(title: &str, margin: i32) -> DynamicImage {
    // main canvas
    let mut img = draw_main_canvas(SLUG_WIDTH, CROP_HEIGHT, (255, 255, 255));

    // font and size
    let font = Font::try_from_bytes(CARD_NAME_FONT).unwrap();
    let scale = Scale::uniform(50.0);

    let (_, th) = drawing::text_size(scale, &font, "E");

    // title
    drawing::draw_text_mut(
        &mut img,
        Rgba([10, 10, 10, 255]),
        15 + margin,
        (CROP_HEIGHT as i32 - th) / 2,
        scale,
        &font,
        title, //.to_uppercase(),
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

fn draw_deck_title(img: &mut RgbaImage, deck: &Deck) -> Result<()> {
    let title = get_title_slug(
        &format!("{} - {}", deck.class, deck.format.to_uppercase()),
        CROP_HEIGHT as i32,
    );
    img.copy_from(&title, MARGIN, MARGIN)?;

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
    get_agent()
        .get(
            &(format!(
                "https://render.worldofwarcraft.com/us/icons/56/classicon_{}.jpg",
                class.to_string().to_lowercase()
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
        .as_ref()
        .ok_or(anyhow!("Card {} has no crop image", card.name))?;

    let mut buf = Vec::new();
    get_agent()
        .get(link)
        .call()?
        .into_reader()
        .read_to_end(&mut buf)?;

    let crop = image::load_from_memory(&buf)?;

    img.copy_from(&crop, CROP_WIDTH, 0)?;

    Ok(())
}

fn build_text_box(text: &str, color: (u8, u8, u8)) -> DynamicImage {
    let plain_font = Font::try_from_bytes(TEXT_PLAIN_FONT).unwrap();
    let bold_font = Font::try_from_bytes(TEXT_BOLD_FONT).unwrap();
    let italic_font = Font::try_from_bytes(TEXT_ITALIC_FONT).unwrap();
    let bold_italic_font = Font::try_from_bytes(TEXT_BOLD_ITALIC_FONT).unwrap();

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

    let text_scale = Scale::uniform(20.0);
    let space_advance = plain_font
        .glyph(' ')
        .scaled(text_scale)
        .h_metrics()
        .advance_width as i32;

    let line_height = drawing::text_size(text_scale, &plain_font, "O").1 * 14 / 10;

    let mut cursor = (15, 10);

    for bx in get_boxes_and_glue(text) {
        let font = match bx.style() {
            TextStyle::Plain => &plain_font,
            TextStyle::Bold => &bold_font,
            TextStyle::Italic => &italic_font,
            TextStyle::BoldItalic => &bold_italic_font,
        };

        let box_advance = drawing::text_size(text_scale, font, &bx.text()).0;

        if SLUG_WIDTH as i32 - 20 <= cursor.0 + box_advance {
            cursor = (15, cursor.1 + line_height);
        }

        drawing::draw_text_mut(
            &mut img,
            Rgba([255, 255, 255, 255]),
            cursor.0,
            cursor.1,
            text_scale,
            font,
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
