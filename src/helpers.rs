use std::fmt::Display;

use colored::Colorize;
use itertools::Either;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till1},
    combinator::{all_consuming, map},
    multi::many1,
    sequence::delimited,
    IResult,
};

// ======
// Prettify
// ======

enum TextTree {
    String(String),
    Bold(Box<TextTree>),
    Italic(Box<TextTree>),
    Seq(Vec<TextTree>),
}
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

// ======
// Either
// ======

pub(crate) fn either<L, R>(cond: bool, left: L, right: R) -> Either<L, R> {
    if cond {
        Either::Left(left)
    } else {
        Either::Right(right)
    }
}
