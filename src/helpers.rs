#![allow(unused)]

use std::fmt::Display;

use colored::Colorize;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till1},
    combinator::{all_consuming, map},
    multi::many1,
    sequence::delimited,
    IResult,
};

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

impl TextTree {
    fn in_bold(self) -> Self {
        Self::Bold(Box::new(self))
    }

    fn in_italic(self) -> Self {
        Self::Italic(Box::new(self))
    }

    fn from_string(input: &str) -> Self {
        Self::String(input.to_owned())
    }

    // Probably not the best way to tackle the problem
    // feels too much.
    fn simplify(self) -> Self {
        let inner_fn = |input| match input {
            Self::Bold(inner) => match *inner {
                Self::Bold(_) => inner.simplify(),
                _ => inner.simplify().in_bold(),
            },
            Self::Italic(inner) => match *inner {
                Self::Italic(_) => inner.simplify(),
                Self::Bold(inner_inner) => inner_inner.simplify().in_italic().in_bold(),
                _ => inner.simplify().in_italic(),
            },
            _ => input,
        };

        let mut simplified = inner_fn(self);
        let mut take_two = inner_fn(simplified.clone());

        while take_two != simplified {
            simplified = take_two;
            take_two = inner_fn(simplified.clone());
        }

        simplified
    }
}

#[cfg(test)]
mod text_tree_tests {
    use super::*;
    use TextTree as TT;

    #[test]
    fn test_simplify_nested_bolds() {
        let case = TT::from_string("5").in_bold().in_bold();
        let case = case.simplify();
        let expected = TT::from_string("5").in_bold();

        assert_eq!(case, expected);
    }

    #[test]
    fn test_simplify_nested_italics() {
        let case = TT::from_string("5").in_italic().in_italic();
        let case = case.simplify();
        let expected = TT::from_string("5").in_italic();

        assert_eq!(case, expected);
    }

    #[test]
    fn test_simplify_nested_bold_italic_bold() {
        let case = TT::from_string("5").in_bold().in_italic().in_bold();
        let case = case.simplify();
        let expected = TT::from_string("5").in_italic().in_bold();

        assert_eq!(case, expected);
    }

    #[test]
    fn test_simplify_nested_bold_italic_bold_italic() {
        let case = TT::from_string("5")
            .in_italic()
            .in_bold()
            .in_italic()
            .in_bold();
        let case = case.simplify();
        let expected = TT::from_string("5").in_italic().in_bold();

        assert_eq!(case, expected);
    }
}

// ====================
// Card Text on Console
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

pub(crate) fn prettify(i: &str) -> String {
    all_consuming(parse_body)(i)
        .map(|(_, s)| s.to_string())
        .unwrap_or(i.to_owned())
}

// ====================
// Card Text on Image
// ====================

enum TextStyle {
    Plain,
    Bold,
    Italic,
    BoldItalic,

    // necessary for some odd cases of nested bolds. either this or simplify
    Black,
    BlackItalic,
}

impl TextStyle {
    fn bold(self) -> Self {
        match self {
            Self::Plain => Self::Bold,
            Self::Bold => Self::Black,
            Self::Italic => Self::BoldItalic,
            Self::BoldItalic => Self::BlackItalic,
            _ => self,
        }
    }

    fn unbold(self) -> Self {
        match self {
            Self::Bold => Self::Plain,
            Self::Black => Self::Bold,
            Self::BoldItalic => Self::Italic,
            Self::BlackItalic => Self::BoldItalic,
            _ => self,
        }
    }

    fn italic(self) -> Self {
        match self {
            Self::Plain => Self::Italic,
            Self::Bold => Self::BoldItalic,
            Self::Black => Self::BlackItalic,
            _ => self,
        }
    }

    fn unitalic(self) -> Self {
        match self {
            Self::Italic => Self::Plain,
            Self::BoldItalic => Self::Bold,
            Self::BlackItalic => Self::Black,
            _ => self,
        }
    }
}

struct TextPiece {
    text: String,
    style: TextStyle,
}
