use itertools::Itertools;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till1},
    combinator::{all_consuming, map, opt},
    multi::many1,
    sequence::delimited,
    IResult,
};
use std::fmt::Write;

#[derive(Debug, PartialEq, Eq, Clone)]
enum TextTree {
    Empty,
    String(String),
    Bold(Box<TextTree>),
    Italic(Box<TextTree>),
    Seq(Vec<TextTree>),
}

pub trait CardTextDisplay {
    fn to_console(&self) -> String;
    fn to_markdown(&self) -> String;
}

impl CardTextDisplay for str {
    fn to_console(&self) -> String {
        use colored::Colorize;

        let mut buffer = String::new();

        for piece in get_text_boxes(self) {
            let Ok(()) = (match piece.style {
                TextStyle::Plain => write!(buffer, "{}", piece.text),
                TextStyle::Bold => write!(buffer, "{}", piece.text.bold()),
                TextStyle::Italic => write!(buffer, "{}", piece.text.italic()),
                TextStyle::BoldItalic => write!(buffer, "{}", piece.text.bold().italic()),
            }) else {
                buffer = self.into();
                break;
            };
        }

        textwrap::fill(
            &buffer,
            textwrap::Options::new(textwrap::termwidth() - 10)
                .initial_indent("\t")
                .subsequent_indent("\t"),
        )
    }

    fn to_markdown(&self) -> String {
        let mut buffer = String::new();

        for piece in get_text_boxes(self) {
            let Ok(()) = (match piece.style {
                TextStyle::Plain => write!(buffer, "{}", piece.text),
                TextStyle::Bold => write!(buffer, "**{}**", piece.text),
                TextStyle::Italic => write!(buffer, "*{}*", piece.text),
                TextStyle::BoldItalic => write!(buffer, "***{}***", piece.text),
            }) else {
                buffer = self.into();
                break;
            };
        }

        buffer
    }
}

// ====================
// Parser from HTML tags to TextTree
// ====================

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
    let parse_fn = map(many1(apply_parsers), |inner| match inner.len() {
        1 => inner.into_iter().next().unwrap(),
        _ => TextTree::Seq(inner),
    });

    // to deal with empty tags: i.e. <b></b>
    map(opt(parse_fn), |c| c.unwrap_or(TextTree::Empty))(i)
}

fn to_text_tree(i: &str) -> Result<TextTree, &str> {
    all_consuming(parse_body)(i).map(|(_, s)| s).map_err(|_| i)
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

    #[test]
    fn test_illidans_gift() -> Result<(), String> {
        let input = "<b>Discover</b> a temporary Fel Barrage, Chaos Strike, or Chaos Nova.<b></b>";
        let case = to_text_tree(dbg!(input))?;
        let expected = TT::Seq(vec![
            TT::in_bold(TT::from_string("Discover")),
            TT::from_string(" a temporary Fel Barrage, Chaos Strike, or Chaos Nova."),
            TT::in_bold(TT::Empty), // This is silly. It should cancel the surrounding tag.
        ]);

        assert_eq!(dbg!(case), expected);
        Ok(())
    }
}

// ====================
// TextTree to Boxes and Glue (originally done for Card text on Images)
// ====================

#[derive(Debug, PartialEq, Eq)]
struct TextPiece {
    text: String,
    style: TextStyle,
}

impl TextPiece {
    fn new(text: &str, style: TextStyle) -> Self {
        Self { text: text.into(), style }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum TextStyle {
    Plain,
    Bold,
    Italic,
    BoldItalic,
}

fn traverse_inner(tree: TextTree, visit: &mut dyn FnMut(TextPiece)) {
    match tree {
        TextTree::Empty => {}
        TextTree::String(text) => visit(TextPiece::new(&text, TextStyle::Plain)),
        TextTree::Bold(inner) => traverse_inner(*inner, &mut |tp| {
            let emboldened = TextPiece {
                style: match tp.style {
                    TextStyle::Plain => TextStyle::Bold,
                    TextStyle::Italic => TextStyle::BoldItalic,
                    _ => tp.style,
                },
                ..tp
            };
            visit(emboldened);
        }),
        TextTree::Italic(inner) => traverse_inner(*inner, &mut |tp| {
            let italicized = TextPiece {
                style: match tp.style {
                    TextStyle::Plain => TextStyle::Italic,
                    TextStyle::Bold => TextStyle::BoldItalic,
                    _ => tp.style,
                },
                ..tp
            };
            visit(italicized);
        }),
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
        tp.text.split_inclusive(' ').map(|t| TextPiece::new(t, tp.style)).collect::<Vec<_>>()
    })
}

fn get_text_boxes(i: &str) -> impl Iterator<Item = TextPiece> {
    let tree = match to_text_tree(i) {
        Ok(inner) => inner,
        Err(text) => TextTree::String(text.to_owned()),
    };

    traverse_text_tree(tree).coalesce(|x, y| {
        if x.style == y.style {
            Ok(TextPiece { text: format!("{}{}", x.text, y.text), style: x.style })
        } else {
            Err((x, y))
        }
    })
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
