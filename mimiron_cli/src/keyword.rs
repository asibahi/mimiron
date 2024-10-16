use anyhow::Result;
use clap::Args;
use mimiron::{
    keyword::lookup,
    localization::{Locale, Localize},
};

#[derive(Args, Clone)]
pub struct KewordArgs {
    input: String,
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run(args: KewordArgs, locale: Locale) -> Result<()> {
    let kws = lookup(&args.input)?;

    for kw in kws {
        println!("{}", kw.in_locale(locale));
    }

    Ok(())
}
