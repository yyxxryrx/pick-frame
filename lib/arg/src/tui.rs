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

#[cfg(test)]
mod tests {
    use super::show_error;
    use crate::lexer::{Span, error::ParseErrorKind, optimize_expr, parse_expr};
    use nom::character::complete::alpha1;

    #[test]
    fn test_show_error() {
        let from = r#"10s - 1.1"#;

        match parse_expr(from.into()) {
            Err(e) => match e {
                nom::Err::Error(err) | nom::Err::Failure(err) => match err.source.code {
                    nom::error::ErrorKind::Count => show_error(
                        "too many args, the time num must lower than 3",
                        &format!("from:{}:{}", err.offset, err.offset + err.length),
                        from,
                        err.offset,
                        err.length,
                        Some("too many args"),
                    ),
                    nom::error::ErrorKind::Tag => match err.kind {
                        ParseErrorKind::Nom => {
                            let word = alpha1::<Span, nom::error::Error<Span>>(err.source.input)
                                .map(|(_, word)| format!(": `{word}`"))
                                .unwrap_or_default();
                            show_error(
                                &format!("invalid token{word}",),
                                &format!("from:{}:{}", err.offset, err.offset + err.length),
                                from,
                                err.offset + err.length,
                                word.len().saturating_sub(4).max(1),
                                Some("invalid token"),
                            );
                        }
                        ParseErrorKind::Op => {
                            show_error(
                                "missing operation, expected `+` or `-`",
                                &format!("from:1:{}", err.offset + 1),
                                from,
                                err.offset,
                                1,
                                Some("here"),
                            );
                        }
                    },
                    nom::error::ErrorKind::Escaped => show_error(
                        &format!(
                            "escaped operation: `{}`",
                            from.chars().nth(err.offset).unwrap_or_default()
                        ),
                        &format!("from:{}:{}", err.offset, err.offset + err.length),
                        from,
                        err.offset,
                        err.length,
                        Some("escaped operation"),
                    ),
                    _ => {}
                },
                _ => {}
            },
            Ok((_, mut expr)) => {
                optimize_expr(&mut expr);
                println!("{expr:?}");
            }
        }
        // show_error(
        //     "too many args, the time num must lower than 3",
        //     "from:7:13",
        //     "end - 1:2:3:4 + 1f",
        //     6,
        //     7,
        //     Some("too many args")
        // );
    }
}
