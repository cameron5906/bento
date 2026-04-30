use console::style;

pub fn success(msg: &str) {
    println!("{} {}", style("✓").green().bold(), msg);
}

pub fn failure(msg: &str) {
    println!("{} {}", style("✗").red().bold(), msg);
}

pub fn info(msg: &str) {
    println!("{} {}", style("·").cyan(), msg);
}

pub fn header(msg: &str) {
    println!("\n{}\n", style(msg).bold().underlined());
}
