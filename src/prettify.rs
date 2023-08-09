use anyhow::{anyhow, Result};
use colored::{ColoredString, Colorize};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_while1},
    combinator::{eof, map},
    multi::many_till,
    sequence::delimited,
    IResult,
};
use std::fmt::Write;

fn parse_bold(i: &str) -> IResult<&str, ColoredString> {
    let second = take_till(|c| c == '<');
    let marks = delimited(tag("<b>"), second, tag("</b>"));
    map(marks, |c: &str| c.bold())(i)
}

fn parse_italic(i: &str) -> IResult<&str, ColoredString> {
    let second = take_till(|c| c == '<');
    let marks = delimited(tag("<i>"), second, tag("</i>"));
    map(marks, |c: &str| c.italic())(i)
}

fn parse_plain(i: &str) -> IResult<&str, ColoredString> {
    let before_marker = take_till(|c| c == '<');
    map(before_marker, |c: &str| c.clear())(i)
}

fn parse_plain2(i: &str) -> IResult<&str, ColoredString> {
    let eol = take_while1(|_| true);
    map(eol, |c: &str| c.clear())(i)
}

fn parse_line(i: &str) -> IResult<&str, Vec<ColoredString>> {
    let fst = alt((parse_bold, parse_italic, parse_plain, parse_plain2));
    let (a, (b, _)) = many_till(fst, eof)(i)?;
    Ok((a, b))
}

fn prettify_inner(input: &str) -> Result<String> {
    // band aid for Eternal Summoner.
    let input = input.replace("<b><b>", "<b>").replace("</b>.</b>", ".</b>"); 

    let mut buffer = String::new();

    let (_, parsed) = parse_line(&input).map_err(|e| anyhow!(format!("{e}")))?;

    for part in parsed {
        write!(buffer, "{part}")?;
    }

    Ok(buffer)
}

pub fn prettify(input: &str) -> String {
    prettify_inner(input).expect("prettify_error")
}
