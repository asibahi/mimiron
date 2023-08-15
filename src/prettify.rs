use anyhow::{anyhow, Result};
use colored::Colorize;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    combinator::{eof, map},
    multi::many_till,
    sequence::delimited,
    IResult,
};

fn parse_bold(i: &str) -> IResult<&str, String> {
    let body = take_until("</b>");
    let marks = delimited(tag("<b>"), body, tag("</b>"));
    map(marks, |c: &str| c.bold().to_string())(i)
}

fn parse_italic(i: &str) -> IResult<&str, String> {
    let body = take_until("</i>");
    let marks = delimited(tag("<i>"), body, tag("</i>"));
    map(marks, |c: &str| c.italic().to_string())(i)
}

fn parse_plain(i: &str) -> IResult<&str, String> {
    let body = alt((take_until("<b>"), take_until("<i>"), take_while1(|_| true)));
    map(body, |c: &str| c.to_owned())(i)
}

fn prettify_inner(input: &str) -> Result<String> {
    let apply_parsers = alt((parse_bold, parse_italic, parse_plain));
    let (_, (parsed, _)) =
        many_till(apply_parsers, eof)(input).map_err(|e| anyhow!(e.to_string()))?;

    let ret = parsed.join("");

    Ok(ret)
}

pub fn prettify(input: &str) -> String {
    let mut pass = input.to_owned();

    while pass.contains("<b>") || pass.contains("<i>") {
        match prettify_inner(&pass) {
            Ok(s) => pass = s,
            Err(_) => return input.to_owned(),
        }
        println!("one pass done");
    }

    pass
}
