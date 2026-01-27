use crate::lexer::{Expr, Span, error::ParseExprResult};
use colored::Colorize;

pub fn show_error(
    message: &str,
    from: &str,
    content: &str,
    offset: usize,
    length: usize,
    tips: Option<&str>,
) {
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
                nom::error::ErrorKind::Count => show_error(
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
                ),
                nom::error::ErrorKind::Tag => match err.kind {
                    ParseErrorKind::Nom => {
                        let word =
                            nom::character::complete::alpha1::<Span, nom::error::Error<Span>>(
                                err.source.input,
                            )
                            .map(|(_, word)| format!(": `{word}`"))
                            .unwrap_or_default();
                        show_error(
                            &format!("invalid token{word}",),
                            &format!(
                                "{content_type}:{}:{}",
                                err.source.input.location_line(),
                                err.offset + 1
                            ),
                            content,
                            err.offset + err.length,
                            word.len().saturating_sub(4).max(1),
                            Some("invalid token"),
                        );
                    }
                    ParseErrorKind::Op => {
                        show_error(
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
                        );
                    }
                },
                nom::error::ErrorKind::Escaped => show_error(
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
