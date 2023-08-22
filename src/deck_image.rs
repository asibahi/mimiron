use anyhow::{anyhow, Context, Result};
use counter::Counter;
use image::{imageops, DynamicImage, GenericImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::{drawing, rect::Rect};
use rayon::prelude::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

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

#[cfg(unix)]
const FONT_DATA: &[u8] = include_bytes!("../data/YanoneKaffeesatz-Medium.ttf");

#[cfg(windows)]
const FONT_DATA: &[u8] = include_bytes!("..\\data\\YanoneKaffeesatz-Medium.ttf");

pub enum DeckImageShape {
    // HSTopDecksStyle
    MultipleColumns,

    // Regular Style
    SingleColumn,
}

pub fn get_deck_image(
    deck: &Deck,
    shape: DeckImageShape,
    agent: ureq::Agent,
) -> Result<DynamicImage> {
    match shape {
        DeckImageShape::MultipleColumns => image_multiple_columns(deck, agent),
        DeckImageShape::SingleColumn => image_single_column(deck, agent),
    }
}

fn image_single_column(deck: &Deck, agent: ureq::Agent) -> Result<DynamicImage> {
    let ordered_cards = deck
        .cards
        .iter()
        .collect::<Counter<_>>()
        .into_iter()
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .collect::<Vec<_>>();

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
    let mut img = ImageBuffer::new(deck_img_width, deck_img_height);
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(deck_img_width, deck_img_height),
        Rgba([255, 255, 255, 255]),
    );

    //  cards
    let title = get_title_slug(&format!("{} - {}", deck.class, deck.format.to_uppercase()))?;
    img.copy_from(&title, MARGIN, MARGIN)?;

    let par_img = Arc::new(Mutex::new(img));

    ordered_cards
        .par_iter()
        .enumerate()
        .try_for_each(|(i, (card, count))| -> Result<()> {
            let i = i as u32 + 1;
            let slug = get_slug(card, *count, &agent)?;
            let mut img = par_img.lock().unwrap();
            img.copy_from(&slug, MARGIN, i * ROW_HEIGHT + MARGIN)?;
            Ok(())
        })?;

    // sideboard cards
    if let Some(sideboards) = &deck.sideboard_cards {
        let mut sb_pos_tracker = ordered_cards.len() + 1;

        for sb in sideboards {
            let sb_start = sb_pos_tracker as u32 * ROW_HEIGHT;
            let sb_title = get_title_slug(&format!("Sideboard: {}", sb.sideboard_card.name))?;
            {
                let mut img = par_img.lock().unwrap();
                img.copy_from(&sb_title, MARGIN, sb_start)?;
            }

            let cards_in_sb = sb
                .cards_in_sideboard
                .iter()
                .collect::<Counter<_>>()
                .into_iter()
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect::<Vec<_>>();

            cards_in_sb.par_iter().enumerate().try_for_each(
                |(i, (card, count))| -> Result<()> {
                    let i = i as u32 + 1;
                    let slug = get_slug(card, *count, &agent)?;
                    let mut img = par_img.lock().unwrap();
                    img.copy_from(&slug, MARGIN, sb_start + i * ROW_HEIGHT)?;
                    Ok(())
                },
            )?;

            sb_pos_tracker += cards_in_sb.len() + 1;
        }
    }

    let img = par_img.lock().unwrap().to_owned();

    Ok(DynamicImage::ImageRgba8(img))
}

fn image_multiple_columns(deck: &Deck, agent: ureq::Agent) -> Result<DynamicImage> {
    let ordered_cards = deck
        .cards
        .iter()
        .collect::<Counter<_>>()
        .into_iter()
        .collect::<BTreeMap<_, _>>();

    let class_cards = ordered_cards
        .iter()
        .filter(|(c, _)| !c.class.contains(&Class::Neutral))
        .collect::<Vec<_>>();

    let neutral_cards = ordered_cards
        .iter()
        .filter(|(c, _)| c.class.contains(&Class::Neutral))
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
    let mut img = ImageBuffer::new(deck_img_width, deck_img_height);
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(deck_img_width, deck_img_height),
        Rgba([255, 255, 255, 255]),
    );

    // class cards
    let class_title = get_title_slug(&format!("{} - {}", deck.class, deck.format.to_uppercase()))?;
    img.copy_from(&class_title, MARGIN, MARGIN)?;

    let par_image = Arc::new(Mutex::new(img));

    class_cards
        .par_iter()
        .enumerate()
        .try_for_each(|(i, (card, count))| -> Result<()> {
            let i = i as u32 + 1;
            let slug = get_slug(card, **count, &agent)?;
            let mut img = par_image.lock().unwrap();
            img.copy_from(&slug, MARGIN, i * ROW_HEIGHT + MARGIN)?;
            Ok(())
        })?;

    // neutral cards
    neutral_cards
        .par_iter()
        .enumerate()
        .try_for_each(|(i, (card, count))| -> Result<()> {
            let mut img = par_image.lock().unwrap();
            if i == 0 {
                let neutrals_title = get_title_slug("Neutrals")?;
                img.copy_from(&neutrals_title, COLUMN_WIDTH + MARGIN, MARGIN)?;
            }

            let i = i as u32 + 1;
            let slug = get_slug(card, **count, &agent)?;
            img.copy_from(&slug, COLUMN_WIDTH + MARGIN, i * ROW_HEIGHT + MARGIN)?;
            Ok(())
        })?;

    // sideboard cards
    if let Some(sideboards) = &deck.sideboard_cards {
        for (sb_i, sb) in sideboards.iter().enumerate() {
            let column_start = COLUMN_WIDTH * (2 + sb_i as u32) + MARGIN;
            let sb_title = get_title_slug(&format!("Sideboard: {}", sb.sideboard_card.name))?;

            {
                let mut img = par_image.lock().unwrap();
                img.copy_from(&sb_title, column_start, MARGIN)?;
            }

            let cards_in_sb = sb
                .cards_in_sideboard
                .iter()
                .collect::<Counter<_>>()
                .into_iter()
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect::<Vec<_>>();

            cards_in_sb.par_iter().enumerate().try_for_each(
                |(i, (card, count))| -> Result<()> {
                    let i = i as u32 + 1;
                    let slug = get_slug(card, *count, &agent)?;
                    let mut img = par_image.lock().unwrap();
                    img.copy_from(&slug, column_start, i * ROW_HEIGHT + MARGIN)?;
                    Ok(())
                },
            )?;
        }
    }

    let img = par_image.lock().unwrap().to_owned();

    Ok(DynamicImage::ImageRgba8(img))
}

pub fn get_slug(card: &Card, count: usize, agent: &ureq::Agent) -> Result<DynamicImage> {
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
    let mut img = ImageBuffer::new(SLUG_WIDTH, CROP_HEIGHT);
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(SLUG_WIDTH, CROP_HEIGHT),
        Rgba([10u8, 10, 10, 255]),
    );

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

    Ok(DynamicImage::ImageRgba8(img))
}

fn get_title_slug(title: &str) -> Result<DynamicImage> {
    // main canvas
    let mut img = ImageBuffer::new(SLUG_WIDTH, CROP_HEIGHT);
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(SLUG_WIDTH, CROP_HEIGHT),
        Rgba([255, 255, 255, 255]),
    );

    // font and size
    let font = rusttype::Font::try_from_bytes(FONT_DATA).unwrap();
    let scale = rusttype::Scale::uniform(50.0);

    let (_, th) = drawing::text_size(scale, &font, "E");

    // title
    drawing::draw_text_mut(
        &mut img,
        Rgba([10, 10, 10, 255]),
        15,
        (CROP_HEIGHT as i32 - th) / 2,
        scale,
        &font,
        title, //.to_uppercase(),
    );

    Ok(DynamicImage::ImageRgba8(img))
}

fn draw_crop_image(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    card: &Card,
    agent: &ureq::Agent,
) -> Result<()> {
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
