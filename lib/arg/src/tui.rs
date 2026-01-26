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
    use nom::character::complete::alpha1;
    use super::show_error;
    use crate::lexer::{parse_expr, Span};

    #[test]
    fn test_show_error() {
        let from = "from - 1:2:3 + 1s +";

        match parse_expr(from.into()) {
            Err(e) => {
                println!("{e:?}");
                match e {
                    nom::Err::Error(err) | nom::Err::Failure(err) => match err.source.code {
                        nom::error::ErrorKind::Count => show_error(
                            "too many args, the time num must lower than 3",
                            &format!("from:{}:{}", err.offset, err.offset + err.length),
                            from,
                            err.offset,
                            err.length,
                            Some("too many args"),
                        ),
                        nom::error::ErrorKind::Tag => {
                            let word = alpha1::<Span, nom::error::Error<Span>>(err.source.input).map(|(_, word)| format!(": `{word}`")).unwrap_or_default();
                            show_error(
                                &format!("invalid token{word}", ),
                                &format!("from:{}:{}", err.offset, err.offset + err.length),
                                from,
                                err.offset + err.length,
                                word.len() - 4,
                                Some("invalid token"),
                            )
                        },
                        _ => {}
                    },
                    _ => {}
                }
            }
            _ => {}
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
