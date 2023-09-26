use anyhow::{anyhow, Context, Result};
use counter::Counter;
use image::{imageops, DynamicImage, GenericImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::{drawing, rect::Rect};
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};

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
    // Each group in its own column. (HS Top Decks)
    Groups,

    // Regular Style over one column
    Single,

    // Regular Style over three columns
    Wide,
}

pub fn get(deck: &Deck, shape: Shape, agent: &ureq::Agent) -> Result<DynamicImage> {
    match shape {
        Shape::Groups => img_groups_format(deck, agent),
        Shape::Wide => img_columns_format(deck, 3, agent),
        Shape::Single => img_columns_format(deck, 1, agent),
    }
}

fn img_columns_format(deck: &Deck, col_count: u32, agent: &ureq::Agent) -> Result<DynamicImage> {
    let (ordered_cards, slug_map) = order_deck_and_get_slugs(deck, agent);

    let deck_img_width = COLUMN_WIDTH * col_count + MARGIN;

    let cards_in_col = {
        let main_deck_length = ordered_cards.len();

        let sideboards_length = deck.sideboard_cards.as_ref().map_or(0, |sbs| {
            sbs.iter()
                .fold(0, |acc, sb| sb.cards_in_sideboard.len() + 1 + acc)
        });

        let length = (main_deck_length + sideboards_length) as u32;

        if length % col_count == 0 {
            length / col_count
        } else {
            length / col_count + 1
        }
    };

    let deck_img_height = (cards_in_col + 1) * ROW_HEIGHT + MARGIN;

    // main canvas
    let mut img = draw_main_canvas(deck_img_width, deck_img_height, (255, 255, 255));

    draw_deck_title(&mut img, deck, agent)?;

    // Main deck
    for (i, (card, _)) in ordered_cards.iter().enumerate() {
        let slug = &slug_map[card];

        let i = i as u32;
        let (col, row) = (i / cards_in_col, i % cards_in_col + 1);

        img.copy_from(slug, col * COLUMN_WIDTH + MARGIN, row * ROW_HEIGHT + MARGIN)?;
    }

    // sideboard cards
    if let Some(sideboards) = &deck.sideboard_cards {
        let mut sb_pos_tracker = ordered_cards.len();

        for sb in sideboards {
            let (col, row) = (
                sb_pos_tracker as u32 / cards_in_col,
                sb_pos_tracker as u32 % cards_in_col + 1,
            );

            img.copy_from(
                &get_title_slug(&format!("Sideboard: {}", sb.sideboard_card.name), 0),
                col * COLUMN_WIDTH + MARGIN,
                row * ROW_HEIGHT + MARGIN,
            )?;
            sb_pos_tracker += 1;

            for slug in order_cards(&sb.cards_in_sideboard)
                .iter()
                .map(| (c, _)| &slug_map[c])
            {
                let i = sb_pos_tracker as u32;
                let (col, row) = (i / cards_in_col, i % cards_in_col + 1);
                img.copy_from(slug, col * COLUMN_WIDTH + MARGIN, row * ROW_HEIGHT + MARGIN)?;

                sb_pos_tracker += 1;
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(img))
}

fn img_groups_format(deck: &Deck, agent: &ureq::Agent) -> Result<DynamicImage> {
    let (ordered_cards, slug_map) = order_deck_and_get_slugs(deck, agent);

    let class_cards = ordered_cards
        .iter()
        .filter_map(|(c, _)| (!c.class.contains(&Class::Neutral)).then(|| &slug_map[c]))
        .enumerate()
        .collect::<Vec<_>>();

    let neutral_cards = ordered_cards
        .iter()
        .filter_map(|(c, _)| c.class.contains(&Class::Neutral).then(|| &slug_map[c]))
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

    draw_deck_title(&mut img, deck, agent)?;
    if !neutral_cards.is_empty() {
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

pub fn get_card_slug(card: &Card, count: usize, agent: &ureq::Agent) -> DynamicImage {
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
    let count = if card.rarity == Rarity::Legendary && count == 1 {
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

fn order_cards(cards: &[Card]) -> BTreeMap<&Card, usize> {
    cards
        .iter()
        .collect::<Counter<_>>()
        .into_iter()
        .collect::<BTreeMap<_, _>>()
}

fn order_deck_and_get_slugs<'d>(
    deck: &'d Deck,
    agent: &ureq::Agent,
) -> (BTreeMap<&'d Card, usize>, HashMap<&'d Card, DynamicImage>) {
    let ordered_cards = order_cards(&deck.cards);
    let ordered_sbs_cards = deck
        .sideboard_cards
        .iter()
        .flat_map(|sbs| {
            sbs.into_iter()
                .flat_map(|sb| order_cards(&sb.cards_in_sideboard))
        })
        .collect::<Vec<_>>();

    // if a card is in two zones it'd have the same slug in both.
    let slug_map = ordered_cards
        .clone()
        .into_par_iter()
        .chain(ordered_sbs_cards.into_par_iter())
        .map(|(card, count)| {
            let slug = get_card_slug(card, count, agent);
            (card, slug)
        })
        .collect::<HashMap<_, _>>();

    (ordered_cards, slug_map)
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

fn draw_deck_title(img: &mut RgbaImage, deck: &Deck, agent: &ureq::Agent) -> Result<()> {
    let title = get_title_slug(
        &format!("{} - {}", deck.class, deck.format.to_uppercase()),
        CROP_HEIGHT as i32,
    );
    img.copy_from(&title, MARGIN, MARGIN)?;

    let class = &deck.class.to_string().to_lowercase();

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
