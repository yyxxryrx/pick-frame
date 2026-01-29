use crate::lexer::{Expr, Span, error::ParseExprResult};
use colored::Colorize;
use std::fmt::Display;

const KEYWORDS: [&str; 3] = ["from", "to", "end"];

pub fn show_error<T>(
    message: &str,
    from: &str,
    content: &str,
    offset: usize,
    length: usize,
    tips: Option<&str>,
    help: Option<T>,
) where
    T: AsRef<str> + Display,
{
    println!("{}: {}", "error".bright_red(), message.bright_white());
    println!("{}", format!("  --> {from}").bright_cyan().bold());
    println!("   {}", "|".bright_cyan().bold());
    println!(" {} {content}", "1 |".bright_cyan().bold());
    println!(
        "   {} {}{} {}",
        "|".bright_cyan().bold(),
        " ".repeat(offset),
        "^".repeat(length).bright_red(),
        tips.unwrap_or_default().bright_red()
    );
    if let Some(help) = help {
        println!("   {}", "|".bright_cyan().bold());
        println!("   {}", format!("= help: {}", help).bright_cyan().bold());
    }
    println!();
}

pub fn handle_error<'a>(
    content: &str,
    content_type: &str,
    res: ParseExprResult<Span<'a>, Expr>,
) -> (Span<'a>, Expr) {
    use crate::lexer::error::ParseErrorKind;
    match res {
        Ok(res) => return res,
        Err(e) => match e {
            nom::Err::Error(err) | nom::Err::Failure(err) => match err.source.code {
                nom::error::ErrorKind::Count => show_error::<&str>(
                    "too many args, the time num must lower than 3",
                    &format!(
                        "{content_type}:{}:{}",
                        err.source.input.location_line(),
                        err.offset + 1
                    ),
                    content,
                    err.offset,
                    err.length,
                    Some("too many args"),
                    None,
                ),
                nom::error::ErrorKind::Tag => match err.kind {
                    ParseErrorKind::Op => {
                        show_error::<&str>(
                            "missing operation, expected `+` or `-`",
                            &format!(
                                "{content_type}:{}:{}",
                                err.source.input.location_line(),
                                err.offset + 1
                            ),
                            content,
                            err.offset,
                            1,
                            Some("here"),
                            None,
                        );
                    }
                    _ => {
                        let word =
                            nom::character::complete::alpha1::<Span, nom::error::Error<Span>>(
                                err.source.input,
                            )
                            .map(|(_, word)| Some(word.to_string()))
                            .unwrap_or(None);
                        let suggests = if let Some(ref word) = word
                            && err.kind == ParseErrorKind::Keywords
                        {
                            let mut temp = KEYWORDS
                                .iter()
                                .map(|words| {
                                    (
                                        words,
                                        strsim::damerau_levenshtein(word, words)
                                            - if words.chars().next() == word.chars().next() {
                                                1
                                            } else {
                                                0
                                            },
                                    )
                                })
                                .filter(|(_, dist)| *dist <= 2)
                                .collect::<Vec<_>>();
                            temp.sort_by(|(_, dist1), (_, dist2)| dist1.cmp(dist2));
                            temp
                        } else {
                            vec![]
                        };
                        let help = if !suggests.is_empty() {
                            match suggests.len() {
                                1 => Some(format!("did you mean `{}`?", suggests[0].0)),
                                x if x > 1 => {
                                    if suggests[0].1 < suggests[1].1 {
                                        Some(format!("did you mean `{}`?", suggests[0].0))
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            }
                        } else {
                            None
                        };
                        let word = word.map(|word| format!(": `{word}`")).unwrap_or_default();
                        show_error(
                            &format!(
                                "{}{word}",
                                if err.kind == ParseErrorKind::Keywords {
                                    "unknown keyword"
                                } else {
                                    "invalid token"
                                }
                            ),
                            &format!(
                                "{content_type}:{}:{}",
                                err.source.input.location_line(),
                                err.offset + 1
                            ),
                            content,
                            err.offset + err.length,
                            word.len().saturating_sub(4).max(1),
                            Some("invalid token"),
                            help.as_ref(),
                        );
                    }
                },
                nom::error::ErrorKind::Escaped => show_error::<&str>(
                    &format!(
                        "escaped operation: `{}`",
                        content.chars().nth(err.offset).unwrap_or_default()
                    ),
                    &format!(
                        "{content_type}:{}:{}",
                        err.source.input.location_line(),
                        err.offset + 1
                    ),
                    content,
                    err.offset,
                    err.length,
                    Some("escaped operation"),
                    None,
                ),
                _ => {}
            },
            _ => {}
        },
    }
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::handle_error;
    use crate::lexer::parse_expr;

    #[test]
    fn test_show_error() {
        let from = r#"end - 1d"#;
        let res = parse_expr(from.into());
        let (_, expr) = handle_error(from, "from", res);
        println!("{expr:?}");
    }
}
