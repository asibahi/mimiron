use itertools::Itertools;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till1},
    combinator::{all_consuming, map},
    multi::many1,
    sequence::delimited,
    IResult,
};
use std::fmt::{Display, Write};

// ====================
// Text Tree
// ====================

#[derive(Debug, PartialEq, Eq, Clone)]
enum TextTree {
    String(String),
    Bold(Box<TextTree>),
    Italic(Box<TextTree>),
    Seq(Vec<TextTree>),
}

// ====================
// Card Text to Console
// ====================

impl Display for TextTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use colored::Colorize;
        match self {
            Self::String(s) => write!(f, "{s}"),
            Self::Bold(b) => write!(f, "{}", b.to_string().bold()),
            Self::Italic(i) => write!(f, "{}", i.to_string().italic()),
            Self::Seq(s) => write!(
                f,
                "{}",
                s.iter().fold(String::new(), |acc, s| acc + &s.to_string())
            ),
        }
    }
}

fn parse_bold(i: &str) -> IResult<&str, TextTree> {
    let marks = delimited(tag("<b>"), parse_body, tag("</b>"));
    map(marks, |c| TextTree::Bold(Box::new(c)))(i)
}

fn parse_italic(i: &str) -> IResult<&str, TextTree> {
    let marks = delimited(tag("<i>"), parse_body, tag("</i>"));
    map(marks, |c| TextTree::Italic(Box::new(c)))(i)
}

fn parse_plain(i: &str) -> IResult<&str, TextTree> {
    let body = take_till1(|c| c == '<');
    map(body, |c: &str| TextTree::String(c.to_owned()))(i)
}

fn parse_body(i: &str) -> IResult<&str, TextTree> {
    let apply_parsers = alt((parse_bold, parse_italic, parse_plain));
    map(many1(apply_parsers), |inner| match inner.len() {
        1 => inner.into_iter().next().unwrap(),
        _ => TextTree::Seq(inner),
    })(i)
}

fn to_text_tree(i: &str) -> Result<TextTree, &str> {
    all_consuming(parse_body)(i).map(|(_, s)| s).map_err(|_| i)
}

pub(crate) fn prettify(i: &str) -> String {
    to_text_tree(i)
        .map(|s| s.to_string())
        .unwrap_or(i.to_owned())
}

#[cfg(test)]
mod prettify_tests {
    use super::*;
    use TextTree as TT;

    impl TextTree {
        fn in_bold(input: Self) -> Self {
            Self::Bold(Box::new(input))
        }

        fn in_italic(input: Self) -> Self {
            Self::Italic(Box::new(input))
        }

        fn from_string(input: &str) -> Self {
            Self::String(input.to_owned())
        }
    }

    #[test]
    fn test_climactic_necrotic_explosion() -> Result<(), String> {
        let input = "<b>Lifesteal</b>. Deal damage. Summon / Souls. <i>(Randomly improved by <b>Corpses</b> you've spent)</i>";
        let case = to_text_tree(dbg!(input))?;
        let expected = TT::Seq(vec![
            TT::in_bold(TT::from_string("Lifesteal")),
            TT::from_string(". Deal damage. Summon / Souls. "),
            TT::in_italic(TT::Seq(vec![
                TT::from_string("(Randomly improved by "),
                TT::in_bold(TT::from_string("Corpses")),
                TT::from_string(" you've spent)"),
            ])),
        ]);

        assert_eq!(dbg!(case), expected);
        Ok(())
    }

    #[test]
    fn test_eternal_summoner() -> Result<(), String> {
        let input = "<b><b>Reborn</b>.</b> <b>Deathrattle:</b> Summon 1 Eternal Knight.";
        let case = to_text_tree(dbg!(input))?;
        let expected = TT::Seq(vec![
            TT::in_bold(TT::Seq(vec![
                TT::in_bold(TT::from_string("Reborn")),
                TT::from_string("."),
            ])),
            TT::from_string(" "),
            TT::in_bold(TT::from_string("Deathrattle:")),
            TT::from_string(" Summon 1 Eternal Knight."),
        ]);

        assert_eq!(dbg!(case), expected);
        Ok(())
    }
}

// ====================
// Card Text on Image
// ====================

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TextPiece {
    text: String,
    style: TextStyle,
}

impl TextPiece {
    pub fn new(text: &str, style: TextStyle) -> Self {
        TextPiece {
            text: text.into(),
            style,
        }
    }

    fn embolden(self) -> Self {
        Self {
            style: match self.style {
                TextStyle::Plain => TextStyle::Bold,
                TextStyle::Italic => TextStyle::BoldItalic,
                _ => self.style,
            },
            ..self
        }
    }

    fn italicize(self) -> Self {
        Self {
            style: match self.style {
                TextStyle::Plain => TextStyle::Italic,
                TextStyle::Bold => TextStyle::BoldItalic,
                _ => self.style,
            },
            ..self
        }
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

    pub fn style(&self) -> TextStyle {
        self.style
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum TextStyle {
    Plain,
    Bold,
    Italic,
    BoldItalic,
}

fn traverse_inner(tree: TextTree, visit: &mut dyn FnMut(TextPiece)) {
    match tree {
        TextTree::String(text) => visit(TextPiece::new(&text, TextStyle::Plain)),
        TextTree::Bold(inner) => traverse_inner(*inner, &mut |tp| visit(tp.embolden())),
        TextTree::Italic(inner) => traverse_inner(*inner, &mut |tp| visit(tp.italicize())),
        TextTree::Seq(seq) => seq.into_iter().for_each(|tt| traverse_inner(tt, visit)),
    }
}

fn traverse_text_tree(tree: TextTree) -> impl Iterator<Item = TextPiece> {
    let mut collector: Vec<TextPiece> = vec![];

    let visit = &mut |tp: TextPiece| match collector.last_mut() {
        Some(last) if last.style == tp.style => last.text.push_str(&tp.text),
        _ => collector.push(tp),
    };

    traverse_inner(tree, visit);

    collector.into_iter().flat_map(|tp| {
        tp.text
            .split_inclusive(' ')
            .map(|t| TextPiece::new(t, tp.style))
            .collect::<Vec<_>>()
    })
}

pub(crate) fn get_boxes_and_glue(i: &str) -> impl Iterator<Item = TextPiece> {
    let tree = match to_text_tree(i) {
        Ok(inner) => inner,
        Err(text) => TextTree::String(text.to_owned()),
    };

    traverse_text_tree(tree)
}

pub fn card_text_to_markdown(i: &str) -> String {
    let mut buffer = String::new();
    let mut prev_style = TextStyle::Plain;

    let boxes = get_boxes_and_glue(i).coalesce(|x, y| {
        if x.style == y.style {
            Ok(TextPiece {
                text: format!("{}{}", x.text, y.text),
                style: x.style,
            })
        } else {
            Err((x, y))
        }
    });

    for piece in boxes {
        let tag = match (prev_style, piece.style) {
            (TextStyle::Plain, TextStyle::Plain)
            | (TextStyle::Bold, TextStyle::Bold)
            | (TextStyle::Italic, TextStyle::Italic)
            | (TextStyle::BoldItalic, TextStyle::BoldItalic) => "",

            (TextStyle::Plain, TextStyle::Italic)
            | (TextStyle::Bold, TextStyle::BoldItalic)
            | (TextStyle::Italic, TextStyle::Plain)
            | (TextStyle::BoldItalic, TextStyle::Bold) => "*",

            (TextStyle::Plain, TextStyle::Bold)
            | (TextStyle::Bold, TextStyle::Plain)
            | (TextStyle::Italic, TextStyle::BoldItalic)
            | (TextStyle::BoldItalic, TextStyle::Italic) => "**",

            (TextStyle::Plain, TextStyle::BoldItalic)
            | (TextStyle::BoldItalic, TextStyle::Plain) => "***",

            (TextStyle::Bold, TextStyle::Italic) => "** *", // should never happen?
            (TextStyle::Italic, TextStyle::Bold) => "* **", // should never happen?
        };

        let Ok(()) = write!(buffer, "{tag}{}", piece.text) else {
            return i.into();
        };

        prev_style = piece.style;
    }

    let Ok(()) = write!(
        buffer,
        "{}",
        match prev_style {
            TextStyle::Plain => "",
            TextStyle::Bold => "**",
            TextStyle::Italic => "*",
            TextStyle::BoldItalic => "***",
        }
    ) else {
        return i.into();
    };

    buffer
}

#[cfg(test)]
mod traverse_tests {
    use super::*;

    use TextPiece as TP;
    use TextStyle as TS;

    #[test]
    fn test_eternal_summoner() -> Result<(), String> {
        let input = "<b><b>Reborn</b>.</b> <b>Deathrattle:</b> Summon 1 Eternal Knight.";
        let tree = to_text_tree(dbg!(input))?;
        let traversal = traverse_text_tree(tree).collect::<Vec<_>>();

        let expected = vec![
            TP::new("Reborn.", TS::Bold),
            TP::new(" ", TS::Plain),
            TP::new("Deathrattle:", TS::Bold),
            TP::new(" ", TS::Plain),
            TP::new("Summon ", TS::Plain),
            TP::new("1 ", TS::Plain),
            TP::new("Eternal ", TS::Plain),
            TP::new("Knight.", TS::Plain),
        ];

        assert_eq!(dbg!(traversal), expected);
        Ok(())
    }

    #[test]
    fn test_climactic_necrotic_explosion() -> Result<(), String> {
        let input = "<b>Lifesteal</b>. Deal damage. Summon / Souls. <i>(Randomly improved by <b>Corpses</b> you've spent)</i>";
        let tree = to_text_tree(dbg!(input))?;
        let traversal = traverse_text_tree(tree).collect::<Vec<_>>();

        let expected = vec![
            TP::new("Lifesteal", TS::Bold),
            TP::new(". ", TS::Plain),
            TP::new("Deal ", TS::Plain),
            TP::new("damage. ", TS::Plain),
            TP::new("Summon ", TS::Plain),
            TP::new("/ ", TS::Plain),
            TP::new("Souls. ", TS::Plain),
            TP::new("(Randomly ", TS::Italic),
            TP::new("improved ", TS::Italic),
            TP::new("by ", TS::Italic),
            TP::new("Corpses", TS::BoldItalic),
            TP::new(" ", TS::Italic),
            TP::new("you've ", TS::Italic),
            TP::new("spent)", TS::Italic),
        ];

        assert_eq!(dbg!(traversal), expected);
        Ok(())
    }
}
