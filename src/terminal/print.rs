use crossterm::style::Stylize;

pub fn warn(text: impl AsRef<str>) {
    println!("{}", text.as_ref().dark_yellow().bold());
}

pub fn question(text: impl AsRef<str>) {
    println!("{}", text.as_ref().italic());
}

pub fn section(text: impl AsRef<str>) {
    println!();
    println!("{}", text.as_ref().underlined());
    println!();
}
