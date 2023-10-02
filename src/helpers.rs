use colored::Colorize;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till1},
    combinator::{all_consuming, map},
    multi::many1,
    sequence::delimited,
    IResult,
};
use std::fmt::Display;

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
    all_consuming(parse_body)(dbg!(i))
        .map(|(_, s)| dbg!(s))
        .map_err(|_| i)
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
    fn new(text: &str, style: TextStyle) -> Self {
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
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TextStyle {
    Plain,
    Bold,
    Italic,
    BoldItalic,
}

/* // ==== Visitor Pattern. Works but reaches "recursion limit".
fn traverse_inner(tree: TextTree, mut visit: impl FnMut(TextPiece)) {
    match tree {
        TextTree::String(text) => visit(TextPiece::new(&text, TextStyle::Plain)),
        TextTree::Bold(inner) => traverse_inner(*inner, |tp| visit(tp.embolden())),
        TextTree::Italic(inner) => traverse_inner(*inner, |tp| visit(tp.italicize())),
        TextTree::Seq(seq) => seq
            .into_iter()
            .for_each(|tt| traverse_inner(tt, |tp| visit(tp))),
    }
} */

#[allow(unused)]
fn traverse_text_tree(tree: TextTree) -> Vec<TextPiece> {
    let mut collector = vec![];

    match tree {
        TextTree::String(text) => collector.push(TextPiece::new(&text, TextStyle::Plain)),
        TextTree::Bold(inner) => traverse_text_tree(*inner)
            .into_iter()
            .for_each(|piece| collector.push(piece.embolden())),
        TextTree::Italic(inner) => traverse_text_tree(*inner)
            .into_iter()
            .for_each(|piece| collector.push(piece.italicize())),
        TextTree::Seq(seq) => seq
            .into_iter()
            .flat_map(traverse_text_tree)
            .for_each(|piece| collector.push(piece)),
    }

    collector
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

        let traversal = traverse_text_tree(tree);

        let expected = vec![
            TP::new("Reborn", TS::Bold),
            TP::new(".", TS::Bold),
            TP::new(" ", TS::Plain),
            TP::new("Deathrattle:", TS::Bold),
            TP::new(" Summon 1 Eternal Knight.", TS::Plain),
        ];

        assert_eq!(dbg!(traversal), expected);
        Ok(())
    }

    #[test]
    fn test_climactic_necrotic_explosion() -> Result<(), String> {
        let input = "<b>Lifesteal</b>. Deal damage. Summon / Souls. <i>(Randomly improved by <b>Corpses</b> you've spent)</i>";
        let tree = to_text_tree(dbg!(input))?;
        let traversal = traverse_text_tree(tree);

        let expected = vec![
            TP::new("Lifesteal", TS::Bold),
            TP::new(". Deal damage. Summon / Souls. ", TS::Plain),
            TP::new("(Randomly improved by ", TS::Italic),
            TP::new("Corpses", TS::BoldItalic),
            TP::new(" you've spent)", TS::Italic),
        ];

        assert_eq!(dbg!(traversal), expected);
        Ok(())
    }
}
