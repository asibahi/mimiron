use crate::AGENT;
use colored::Colorize;
use serde::Deserialize;
use std::fmt::{Display, Formatter};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsArticle {
    pub title: String,
    pub default_url: String,
    // pub header: Url,
    pub thumbnail: Url,
    pub summary: String,
}

impl Display for NewsArticle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "\t{}\n{}\n{:>8}: {}",
            self.title.bold().green(),
            textwrap::fill(
                &self.summary,
                textwrap::Options::new(textwrap::termwidth() - 10)
                    .initial_indent(&format!("{:>8}: ", "Summary".bold()))
                    .subsequent_indent("          "),
            ),
            "Link".bold(),
            self.default_url.underline().bright_blue()
        )
    }
}

#[derive(Debug, Deserialize)]
// #[serde(transparent)]
pub struct Url {
    pub url: String,
}

pub fn get_news<'a>() -> anyhow::Result<impl Iterator<Item = NewsArticle> + 'a> {
    let ret = AGENT
        .get("https://hearthstone.blizzard.com/en-us/api/blog/articleList/")
        .query_pairs([("page", "1"), ("pageSize", "12")])
        .call()
        .map_err(|_| anyhow::anyhow!("Unable to get news"))?
        .body_mut()
        .read_json::<Vec<NewsArticle>>()
        .map_err(|_| anyhow::anyhow!("Unable to parse news"))?;

    let iter = ret.into_iter();

    Ok(iter)
}
