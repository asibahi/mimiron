use anyhow::{anyhow, Result};
use colored::{ColoredString, Colorize};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    combinator::{eof, map},
    multi::many_till,
    sequence::delimited,
    IResult,
};

fn parse_bold(i: &str) -> IResult<&str, ColoredString> {
    let body = take_until("</b>");
    let marks = delimited(tag("<b>"), body, tag("</b>"));
    map(marks, |c: &str| c.bold())(i)
}

fn parse_italic(i: &str) -> IResult<&str, ColoredString> {
    let body = take_until("</i>");
    let marks = delimited(tag("<i>"), body, tag("</i>"));
    map(marks, |c: &str| c.italic())(i)
}

fn parse_plain(i: &str) -> IResult<&str, ColoredString> {
    let body = alt((take_until("<b>"), take_until("<i>"), take_while1(|_| true)));
    map(body, |c: &str| c.clear())(i)
}

fn prettify_inner(input: &str) -> Result<String> {
    let apply_parsers = alt((parse_bold, parse_italic, parse_plain));
    let (_, (parsed, _)) =
        many_till(apply_parsers, eof)(&input).map_err(|e| anyhow!(e.to_string()))?;

    let ret = parsed
        .into_iter()
        .fold(String::new(), |acc, s| format!("{acc}{s}"));

    Ok(ret)
}

pub fn prettify(input: &str) -> String {
    // band aid for Eternal Summoner:   <b><b>Reborn</b>.</b> <b>Deathrattle:</b> Summon 1 Eternal Knight.
    let input = input.replace("<b><b>", "<b>").replace("</b>.</b>", ".</b>");

    match prettify_inner(&input) {
        Ok(ret) => ret,
        Err(_) => input.to_owned(),
    }
}
