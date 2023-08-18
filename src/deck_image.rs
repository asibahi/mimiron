use anyhow::{anyhow, Result};
use counter::Counter;
use image::{imageops, DynamicImage, GenericImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::{drawing, rect::Rect};
use std::collections::BTreeMap;

use crate::{
    card::Card,
    card_details::{Class, Rarity},
    deck::Deck,
};

//  Numbers based on the crops provided by Blizzard API
const CROP_WIDTH: u32 = 243;
const CROP_HEIGHT: u32 = 64;
const SLUG_WIDTH: u32 = CROP_WIDTH * 2 + CROP_HEIGHT;
const MARGIN: u32 = 5;

const FONT_DATA: &[u8] = include_bytes!("../data/YanoneKaffeesatz-Medium.ttf");

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
        .collect::<BTreeMap<_, _>>();

    let deck_img_width = MARGIN * 2 + SLUG_WIDTH;

    let deck_img_height = {
        let main_deck_length = ordered_cards.len() + 1;

        let sideboards_length = deck.sideboard_cards.as_ref().map_or(0, |sbs| {
            sbs.iter()
                .fold(0, |acc, sb| sb.cards_in_sideboard.len() + 1 + acc)
        });

        let length = main_deck_length + sideboards_length;

        MARGIN + (length as u32 * (MARGIN + CROP_HEIGHT))
    };

    // main canvas
    let mut img = ImageBuffer::new(deck_img_width, deck_img_height);
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(deck_img_width, deck_img_height),
        Rgba([255, 255, 255, 255]),
    );

    //  cards
    let title = get_title_slug(format!("{} - {}", deck.class, deck.format.to_uppercase()))?;
    img.copy_from(&title, MARGIN, MARGIN)?;

    for (i, (card, count)) in ordered_cards.iter().enumerate() {
        let slug = get_slug(card, *count, agent.clone())?;
        let i = 1 + i as u32;

        img.copy_from(&slug, MARGIN, MARGIN + i * (MARGIN + CROP_HEIGHT))?;
    }

    // sideboard cards
    if let Some(sideboards) = &deck.sideboard_cards {
        let mut sb_pos_tracker = ordered_cards.len() + 1;

        for sb in sideboards {
            let sb_start = sb_pos_tracker as u32 * (CROP_HEIGHT + MARGIN);
            let sb_title = get_title_slug(format!("Sideboard: {}", sb.sideboard_card.name))?;
            img.copy_from(&sb_title, MARGIN, sb_start)?;

            let cards_in_sb = sb
                .cards_in_sideboard
                .iter()
                .collect::<Counter<_>>()
                .into_iter()
                .collect::<BTreeMap<_, _>>();

            for (i, (card, count)) in cards_in_sb.iter().enumerate() {
                let slug = get_slug(card, *count, agent.clone())?;
                let i = 1 + i as u32;

                img.copy_from(&slug, MARGIN, sb_start + i * (MARGIN + CROP_HEIGHT))?;
            }

            sb_pos_tracker += cards_in_sb.len() + 1;
        }
    }

    Ok(DynamicImage::ImageRgba8(img))
}

fn image_multiple_columns(deck: &Deck, agent: ureq::Agent) -> Result<DynamicImage> {
    let class_cards = deck
        .cards
        .iter()
        .filter(|c| !c.class.contains(&Class::Neutral))
        .collect::<Counter<_>>()
        .into_iter()
        .collect::<BTreeMap<_, _>>();

    let neutral_cards = deck
        .cards
        .iter()
        .filter(|c| c.class.contains(&Class::Neutral))
        .collect::<Counter<_>>()
        .into_iter()
        .collect::<BTreeMap<_, _>>();

    // deck image width
    // assumes decks will always have class cards
    let column_width = MARGIN + SLUG_WIDTH;

    let deck_img_width = {
        let mut columns = 1;
        if !neutral_cards.is_empty() {
            columns += 1;
        }
        if let Some(sideboards) = &deck.sideboard_cards {
            columns += sideboards.len();
        }
        let columns = columns as u32;

        MARGIN + columns * column_width
    };

    // deck image height
    // ignores length of sideboards. unlikely to be larger than either class_cards or neutral_cards
    let deck_img_height = {
        let length = 1 + class_cards.len().max(neutral_cards.len()) as u32;
        MARGIN + (length * (MARGIN + CROP_HEIGHT))
    };

    // main canvas
    let mut img = ImageBuffer::new(deck_img_width, deck_img_height);
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(deck_img_width, deck_img_height),
        Rgba([255, 255, 255, 255]),
    );

    // class cards
    let class_title = get_title_slug(format!("{} - {}", deck.class, deck.format.to_uppercase()))?;
    img.copy_from(&class_title, MARGIN, MARGIN)?;

    for (i, (card, count)) in class_cards.into_iter().enumerate() {
        let slug = get_slug(card, count, agent.clone())?;
        let i = 1 + i as u32;

        img.copy_from(&slug, MARGIN, MARGIN + i * (MARGIN + CROP_HEIGHT))?;
    }

    // neutral cards
    for (i, (card, count)) in neutral_cards.into_iter().enumerate() {
        if i == 0 {
            let neutrals_title = get_title_slug(String::from("Neutrals"))?;
            img.copy_from(&neutrals_title, column_width + MARGIN, MARGIN)?;
        }

        let slug = get_slug(card, count, agent.clone())?;
        let i = 1 + i as u32;

        img.copy_from(
            &slug,
            column_width + MARGIN,
            MARGIN + i * (MARGIN + CROP_HEIGHT),
        )?;
    }

    // sideboard cards
    if let Some(sideboards) = &deck.sideboard_cards {
        for (sb_i, sb) in sideboards.iter().enumerate() {
            let column_start = column_width * (2 + sb_i as u32) + MARGIN;
            let sb_title = get_title_slug(format!("Sideboard: {}", sb.sideboard_card.name))?;
            img.copy_from(&sb_title, column_start, MARGIN)?;

            for (i, (card, count)) in sb
                .cards_in_sideboard
                .iter()
                .collect::<Counter<_>>()
                .into_iter()
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .enumerate()
            {
                let slug = get_slug(card, count, agent.clone())?;
                let i = 1 + i as u32;

                img.copy_from(&slug, column_start, MARGIN + i * (MARGIN + CROP_HEIGHT))?;
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(img))
}

pub fn get_slug(card: &Card, count: usize, agent: ureq::Agent) -> Result<DynamicImage> {
    assert!(count > 0);

    let name = &card.name;

    // colors from https://wowpedia.fandom.com/wiki/Quality
    let r_color = match &card.rarity {
        Rarity::Legendary => (255, 128, 0),
        Rarity::Epic => (163, 53, 238),
        Rarity::Rare => (0, 112, 221),
        _ => (157, 157, 157),
    };

    let link = card
        .crop_image
        .clone()
        .ok_or(anyhow!("No crop image for {}", name))?;

    let mut buf = Vec::new();
    agent
        .get(&link)
        .call()?
        .into_reader()
        .read_to_end(&mut buf)?;

    let crop = image::load_from_memory(&buf)?;

    // main canvas
    let mut img = ImageBuffer::new(SLUG_WIDTH, CROP_HEIGHT);
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(SLUG_WIDTH, CROP_HEIGHT),
        Rgba([10u8, 10, 10, 255]),
    );

    img.copy_from(&crop, CROP_WIDTH, 0)?;

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
    let (font, scale) = get_font_and_scale()?;

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

pub fn get_title_slug(name: String) -> Result<DynamicImage> {
    // main canvas
    let mut img = ImageBuffer::new(SLUG_WIDTH, CROP_HEIGHT);
    drawing::draw_filled_rect_mut(
        &mut img,
        Rect::at(0, 0).of_size(SLUG_WIDTH, CROP_HEIGHT),
        Rgba([255, 255, 255, 255]),
    );

    // font and size
    let (font, scale) = get_font_and_scale()?;

    // title
    drawing::draw_text_mut(
        &mut img,
        Rgba([10, 10, 10, 255]),
        CROP_HEIGHT as i32 + 10,
        15,
        scale,
        &font,
        &name, //.to_uppercase(),
    );

    Ok(DynamicImage::ImageRgba8(img))
}

#[inline]
fn get_font_and_scale() -> Result<(rusttype::Font<'static>, rusttype::Scale)> {
    let font = rusttype::Font::try_from_bytes(FONT_DATA).ok_or(anyhow!("couldn't load font"))?;
    let scale = rusttype::Scale::uniform(40.0);
    Ok((font, scale))
}
