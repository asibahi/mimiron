use nom::{
    Parser,
    branch::alt,
    bytes::{tag, take_till1},
    combinator::all_consuming,
    multi::many0,
    sequence::delimited,
};
use std::{borrow::Cow, fmt::Write};

#[derive(Debug, PartialEq, Eq, Clone)]
enum TextTree<'s> {
    Empty,
    String(&'s str),
    Bold(Box<TextTree<'s>>),
    Italic(Box<TextTree<'s>>),
    Seq(Vec<TextTree<'s>>),
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

macro_rules! parser {
    ($name: ident, $expr: expr) => {
        struct $name;
        impl<'a> Parser<&'a str> for $name {
            type Output = TextTree<'a>;
            type Error = ();

            fn process<OM: nom::OutputMode>(
                &mut self,
                input: &'a str,
            ) -> nom::PResult<OM, &'a str, Self::Output, Self::Error> {
                $expr.process::<OM>(input)
            }
        }
    };
}

parser!(Plain, take_till1(|c| c == '<').map(TextTree::String));
parser!(
    Bold,
    delimited(tag("<b>"), Body, tag("</b>")).map(|c| TextTree::Bold(Box::new(c)))
);
parser!(
    Italic,
    delimited(tag("<i>"), Body, tag("</i>")).map(|c| TextTree::Italic(Box::new(c)))
);
parser!(
    Body,
    many0(alt((Bold, Italic, Plain))).map(|inner| match inner.len() {
        0 => TextTree::Empty, // to deal with empty tags: i.e. <b></b>
        1 => inner.into_iter().next().unwrap(),
        _ => TextTree::Seq(inner),
    })
);

fn to_text_tree(i: &str) -> Result<TextTree<'_>, &str> {
    all_consuming(Body)
        .parse_complete(i)
        .map(|(_, s)| s)
        .map_err(|_| i)
}

#[cfg(test)]
mod prettify_tests {
    use super::*;
    use TextTree as TT;

    impl TextTree<'_> {
        fn in_bold(input: Self) -> Self {
            Self::Bold(Box::new(input))
        }

        fn in_italic(input: Self) -> Self {
            Self::Italic(Box::new(input))
        }
    }

    macro_rules! test {
        ($name:ident, $text:literal, $expected:expr) => {
            #[test]
            fn $name() {
                let case = to_text_tree($text).unwrap();
                assert_eq!(case, $expected);
            }
        };
    }

    test!(
        test_climactic_necrotic_explosion,
        "<b>Lifesteal</b>. Deal damage. Summon / Souls. <i>(Randomly improved by <b>Corpses</b> you've spent)</i>",
        TT::Seq(vec![
            TT::in_bold(TT::String("Lifesteal")),
            TT::String(". Deal damage. Summon / Souls. "),
            TT::in_italic(TT::Seq(vec![
                TT::String("(Randomly improved by "),
                TT::in_bold(TT::String("Corpses")),
                TT::String(" you've spent)"),
            ])),
        ])
    );

    test!(
        test_eternal_summoner,
        "<b><b>Reborn</b>.</b> <b>Deathrattle:</b> Summon 1 Eternal Knight.",
        TT::Seq(vec![
            TT::in_bold(TT::Seq(vec![
                TT::in_bold(TT::String("Reborn")),
                TT::String("."),
            ])),
            TT::String(" "),
            TT::in_bold(TT::String("Deathrattle:")),
            TT::String(" Summon 1 Eternal Knight."),
        ])
    );

    test!(
        test_illidans_gift,
        "<b>Discover</b> a temporary Fel Barrage, Chaos Strike, or Chaos Nova.<b></b>",
        TT::Seq(vec![
            TT::in_bold(TT::String("Discover")),
            TT::String(" a temporary Fel Barrage, Chaos Strike, or Chaos Nova."),
            TT::in_bold(TT::Empty), // This is silly. It should cancel the surrounding tag.
        ])
    );
}

// ====================
// TextTree to Boxes and Glue (originally done for Card text on Images)
// ====================

#[derive(Debug, PartialEq, Eq)]
struct TextPiece<'s> {
    text: Cow<'s, str>,
    style: TextStyle,
}

impl<'s> TextPiece<'s> {
    fn new(
        text: &'s str,
        style: TextStyle,
    ) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum TextStyle {
    Plain,
    Bold,
    Italic,
    BoldItalic,
}

fn traverse_inner<'s>(
    tree: TextTree<'s>,
    visit: &mut dyn FnMut(TextPiece<'s>),
) {
    match tree {
        TextTree::Empty => {}
        TextTree::String(text) => visit(TextPiece::new(text, TextStyle::Plain)),
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

fn traverse_text_tree<'s>(tree: TextTree<'s>) -> impl Iterator<Item = TextPiece<'s>> {
    let mut collector: Vec<TextPiece<'s>> = vec![];

    let mut visit = |tp: TextPiece<'s>| match collector.last_mut() {
        Some(last) if last.style == tp.style || tp.text.trim().is_empty() => {
            last.text.to_mut().push_str(&tp.text)
        }
        _ => collector.push(tp),
    };

    traverse_inner(tree, &mut visit);

    collector.into_iter()
}

fn get_text_boxes(i: &str) -> impl Iterator<Item = TextPiece<'_>> {
    let tree = match to_text_tree(i) {
        Ok(inner) => inner,
        Err(text) => TextTree::String(text),
    };

    traverse_text_tree(tree)
}

#[cfg(test)]
mod traverse_tests {
    use super::*;

    use TextPiece as TP;
    use TextStyle as TS;

    macro_rules! test {
        ($name:ident, $text:literal, $expected:expr $(,)?) => {
            #[test]
            fn $name() {
                let case = get_text_boxes($text);
                assert!(case.eq($expected));
            }
        };
    }

    test!(
        test_climactic_necrotic_explosion,
        "<b>Lifesteal</b>. Deal damage. Summon / Souls. <i>(Randomly improved by <b>Corpses</b> you've spent)</i>",
        vec![
            TP::new("Lifesteal", TS::Bold),
            TP::new(". Deal damage. Summon / Souls. ", TS::Plain),
            TP::new("(Randomly improved by ", TS::Italic),
            TP::new("Corpses", TS::BoldItalic),
            TP::new(" you've spent)", TS::Italic),
        ]
    );

    test!(
        test_eternal_summoner,
        "<b><b>Reborn</b>.</b> <b>Deathrattle:</b> Summon 1 Eternal Knight.",
        vec![
            TP::new("Reborn. Deathrattle:", TS::Bold),
            TP::new(" Summon 1 Eternal Knight.", TS::Plain),
        ]
    );

    test!(
        test_illidans_gift,
        "<b>Discover</b> a temporary Fel Barrage, Chaos Strike, or Chaos Nova.<b></b>",
        vec![
            TP::new("Discover", TS::Bold),
            TP::new(
                " a temporary Fel Barrage, Chaos Strike, or Chaos Nova.",
                TS::Plain
            ),
        ]
    );
}
