use anyhow::{anyhow, Result};
use colored::Colorize;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    combinator::{eof, map, rest},
    multi::many_till,
    sequence::delimited,
    IResult,
};

const B_BEG: &str = "<b>";
const B_END: &str = "</b>";
const I_BEG: &str = "<i>";
const I_END: &str = "</i>";

fn parse_bold(i: &str) -> IResult<&str, String> {
    let body = take_until(B_END);
    let marks = delimited(tag(B_BEG), body, tag(B_END));
    map(marks, |c: &str| c.bold().to_string())(i)
}

fn parse_italic(i: &str) -> IResult<&str, String> {
    let body = take_until(I_END);
    let marks = delimited(tag(I_BEG), body, tag(I_END));
    map(marks, |c: &str| c.italic().to_string())(i)
}

fn parse_plain(i: &str) -> IResult<&str, String> {
    let body = alt((take_until(B_BEG), take_until(I_BEG), rest));
    map(body, |c: &str| c.to_owned())(i)
}

fn prettify_inner(input: &str) -> Result<String> {
    let apply_parsers = alt((parse_bold, parse_italic, parse_plain));
    let (_, (parsed, _)) =
        many_till(apply_parsers, eof)(input).map_err(|e| anyhow!(e.to_string()))?;

    let ret = parsed.join("");

    Ok(ret)
}

pub(crate) fn prettify(input: &str) -> String {
    let mut pass = input.to_owned();

    while pass.contains(B_BEG) || pass.contains(I_BEG) {
        match prettify_inner(&pass) {
            Ok(s) => pass = s,
            Err(_) => break,
        }
    }

    pass
}
