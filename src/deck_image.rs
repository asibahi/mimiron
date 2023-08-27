use anyhow::{anyhow, Context, Result};
use counter::Counter;
use image::{imageops, DynamicImage, GenericImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::{drawing, rect::Rect};
use rayon::prelude::*;
use std::{collections::BTreeMap, sync::Mutex};

use crate::{
    card::Card,
    card_details::{Class, Rarity},
    deck::Deck,
};

//  Numbers based on the crops provided by Blizzard API
const CROP_WIDTH: u32 = 243;
const CROP_HEIGHT: u32 = 64;

const MARGIN: u32 = 5;

const SLUG_WIDTH: u32 = CROP_WIDTH * 2 + CROP_HEIGHT;
const ROW_HEIGHT: u32 = CROP_HEIGHT + MARGIN;
const COLUMN_WIDTH: u32 = SLUG_WIDTH + MARGIN;

const FONT_DATA: &[u8] = include_bytes!("../data/YanoneKaffeesatz-Medium.ttf");

pub enum Shape {
    // HSTopDecksStyle
    MultipleColumns,

    // Regular Style
    SingleColumn,
}

pub fn get(deck: &Deck, shape: Shape, agent: &ureq::Agent) -> Result<DynamicImage> {
    match shape {
        Shape::MultipleColumns => image_multiple_columns(deck, agent),
        Shape::SingleColumn => image_single_column(deck, agent),
    }
}

fn image_single_column(deck: &Deck, agent: &ureq::Agent) -> Result<DynamicImage> {
    let ordered_cards = order_cards(&deck.cards);

    let deck_img_width = MARGIN * 2 + SLUG_WIDTH;

    let deck_img_height = {
        let main_deck_length = ordered_cards.len() + 1;

        let sideboards_length = deck.sideboard_cards.as_ref().map_or(0, |sbs| {
            sbs.iter()
                .fold(0, |acc, sb| sb.cards_in_sideboard.len() + 1 + acc)
        });

        let length = main_deck_length + sideboards_length;

        (length as u32 * ROW_HEIGHT) + MARGIN
    };

    // main canvas
    let mut img = draw_main_canvas(deck_img_width, deck_img_height, (255, 255, 255));

    // cards
    draw_deck_title(&mut img, deck, agent)?;

    let par_img = Mutex::new(img);

    ordered_cards
        .par_iter()
        .try_for_each(|(i, (card, count))| -> Result<()> {
            let i = *i as u32 + 1;
            let slug = get_slug(card, *count, agent);
            let mut img = par_img.lock().unwrap();
            img.copy_from(&slug, MARGIN, i * ROW_HEIGHT + MARGIN)?;
            Ok(())
        })?;

    // sideboard cards
    if let Some(sideboards) = &deck.sideboard_cards {
        let mut sb_pos_tracker = ordered_cards.len() + 1;

        for sb in sideboards {
            let sb_start = sb_pos_tracker as u32 * ROW_HEIGHT;
            '_mutex_block: {
                let sb_title = get_title_slug(&format!("Sideboard: {}", sb.sideboard_card.name), 0);
                let mut img = par_img.lock().unwrap();
                img.copy_from(&sb_title, MARGIN, sb_start)?;
            }

            let cards_in_sb = order_cards(&sb.cards_in_sideboard);

            sb_pos_tracker += cards_in_sb.len() + 1;

            cards_in_sb
                .into_par_iter()
                .try_for_each(|(i, (card, count))| -> Result<()> {
                    let i = i as u32 + 1;
                    let slug = get_slug(card, count, agent);
                    let mut img = par_img.lock().unwrap();
                    img.copy_from(&slug, MARGIN, sb_start + i * ROW_HEIGHT)?;
                    Ok(())
                })?;
        }
    }

    let img = par_img.lock().unwrap().to_owned();

    Ok(DynamicImage::ImageRgba8(img))
}

fn image_multiple_columns(deck: &Deck, agent: &ureq::Agent) -> Result<DynamicImage> {
    let ordered_cards = deck
        .cards
        .iter()
        .collect::<Counter<_>>()
        .into_iter()
        .collect::<BTreeMap<_, _>>();

    let class_cards = ordered_cards
        .iter()
        .filter(|(c, _)| !c.class.contains(&Class::Neutral))
        .enumerate()
        .collect::<Vec<_>>();

    let neutral_cards = ordered_cards
        .iter()
        .filter(|(c, _)| c.class.contains(&Class::Neutral))
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
        let columns = columns as u32;

        columns * COLUMN_WIDTH + MARGIN
    };

    // deck image height
    // ignores length of sideboards. unlikely to be larger than either class_cards or neutral_cards
    let deck_img_height = {
        let length = 1 + class_cards.len().max(neutral_cards.len()) as u32;
        (length * ROW_HEIGHT) + MARGIN
    };

    // main canvas
    let mut img = draw_main_canvas(deck_img_width, deck_img_height, (255, 255, 255));

    draw_deck_title(&mut img, deck, agent)?;

    // class cards
    let par_image = Mutex::new(img);

    class_cards
        .into_par_iter()
        .try_for_each(|(i, (card, count))| -> Result<()> {
            let i = i as u32 + 1;
            let slug = get_slug(card, *count, agent);
            let mut img = par_image.lock().unwrap();
            img.copy_from(&slug, MARGIN, i * ROW_HEIGHT + MARGIN)?;
            Ok(())
        })?;

    // neutral cards
    neutral_cards
        .into_par_iter()
        .try_for_each(|(i, (card, count))| -> Result<()> {
            let slug = get_slug(card, *count, agent);

            let mut img = par_image.lock().unwrap();
            if i == 0 {
                let neutrals_title = get_title_slug("Neutrals", 0);
                img.copy_from(&neutrals_title, COLUMN_WIDTH + MARGIN, MARGIN)?;
            }
            let i = i as u32 + 1;
            img.copy_from(&slug, COLUMN_WIDTH + MARGIN, i * ROW_HEIGHT + MARGIN)?;
            Ok(())
        })?;

    // sideboard cards
    if let Some(sideboards) = &deck.sideboard_cards {
        for (sb_i, sb) in sideboards.iter().enumerate() {
            let column_start = COLUMN_WIDTH * (2 + sb_i as u32) + MARGIN;
            '_mutex_block: {
                let sb_title = get_title_slug(&format!("Sideboard: {}", sb.sideboard_card.name), 0);
                let mut img = par_image.lock().unwrap();
                img.copy_from(&sb_title, column_start, MARGIN)?;
            }

            let cards_in_sb = order_cards(&sb.cards_in_sideboard);

            cards_in_sb
                .into_par_iter()
                .try_for_each(|(i, (card, count))| -> Result<()> {
                    let i = i as u32 + 1;
                    let slug = get_slug(card, count, agent);
                    let mut img = par_image.lock().unwrap();
                    img.copy_from(&slug, column_start, i * ROW_HEIGHT + MARGIN)?;
                    Ok(())
                })?;
        }
    }

    let img = par_image.lock().unwrap().to_owned();

    Ok(DynamicImage::ImageRgba8(img))
}

fn order_cards(cards: &[Card]) -> Vec<(usize, (&Card, usize))> {
    cards
        .iter()
        .collect::<Counter<_>>()
        .into_iter()
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .enumerate()
        .collect::<Vec<_>>()
}

pub fn get_slug(card: &Card, count: usize, agent: &ureq::Agent) -> DynamicImage {
    assert!(count > 0);

    let name = &card.name;

    // colors from https://wowpedia.fandom.com/wiki/Quality
    let r_color = match &card.rarity {
        Rarity::Legendary => (255, 128, 0),
        Rarity::Epic => (163, 53, 238),
        Rarity::Rare => (0, 112, 221),
        _ => (157, 157, 157),
    };

    // main canvas
    let mut img = draw_main_canvas(SLUG_WIDTH, CROP_HEIGHT, (10, 10, 10));

    if let Err(e) = draw_crop_image(&mut img, card, agent) {
        eprintln!("Failed to get image of {}: {e}", card.name);
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
    let font = rusttype::Font::try_from_bytes(FONT_DATA).unwrap();
    let scale = rusttype::Scale::uniform(40.0);

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
    let count = if count == 1 && card.rarity == Rarity::Legendary {
        String::new()
    } else {
        count.to_string()
    };
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

fn get_title_slug(title: &str, margin: i32) -> DynamicImage {
    // main canvas
    let mut img = draw_main_canvas(SLUG_WIDTH, CROP_HEIGHT, (255, 255, 255));

    // font and size
    let font = rusttype::Font::try_from_bytes(FONT_DATA).unwrap();
    let scale = rusttype::Scale::uniform(50.0);

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

fn draw_deck_title(
    img: &mut RgbaImage,
    deck: &Deck,
    agent: &ureq::Agent,
) -> Result<(), anyhow::Error> {
    let title = get_title_slug(
        &format!("{} - {}", deck.class, deck.format.to_uppercase()),
        CROP_HEIGHT as i32,
    );
    img.copy_from(&title, MARGIN, MARGIN)?;
    draw_class_icon(img, &deck.class, agent).ok();
    Ok(())
}

fn draw_class_icon(img: &mut RgbaImage, class: &Class, agent: &ureq::Agent) -> Result<()> {
    let class = class.to_string().to_lowercase();

    let class_img = {
        let mut buf = Vec::new();
        agent
            .get(&(format!("https://render.worldofwarcraft.com/us/icons/56/classicon_{class}.jpg")))
            .call()
            .with_context(|| "Could not connect to class image link")?
            .into_reader()
            .read_to_end(&mut buf)
            .with_context(|| "Could not read  class image link")?;
        image::load_from_memory(&buf)?
    }
    .resize_to_fill(CROP_HEIGHT, CROP_HEIGHT, imageops::FilterType::Gaussian);

    img.copy_from(&class_img, MARGIN, MARGIN)?;

    Ok(())
}

fn draw_crop_image(img: &mut RgbaImage, card: &Card, agent: &ureq::Agent) -> Result<()> {
    let link = card
        .crop_image
        .as_ref()
        .ok_or(anyhow!("Card {} has no crop image", card.name))?;

    let mut buf = Vec::new();
    agent
        .get(link)
        .call()
        .with_context(|| "Could not connect to image link")?
        .into_reader()
        .read_to_end(&mut buf)
        .with_context(|| "Could not read image link")?;

    let crop = image::load_from_memory(&buf)?;

    img.copy_from(&crop, CROP_WIDTH, 0)?;

    Ok(())
}
