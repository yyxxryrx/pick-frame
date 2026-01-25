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

    #[test]
    fn test_show_error() {
        show_error(
            "too many args, the time num must lower than 3",
            "from:7:13",
            "end - 1:2:3:4 + 1f",
            6,
            7,
            Some("too many args")
        );
    }
}