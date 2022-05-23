use log::{Level, LevelFilter};
use colored::{Color, Colorize};

/// Setup logging system
pub fn setup_logger(level: LevelFilter) -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            let (first, rest, color) = format_log_message(
                message.to_string(),
                record.level()
            );
            out.finish(format_args!(
                "{:>12} {}",
                first.bold().color(color),
                rest
            ))
        })
        .level(level)
        .chain(std::io::stderr())
        .apply()?;
    Ok(())
}

pub fn format_log_message(msg: String, level: Level) -> (String, String, Color) {
    match level {
        Level::Error => ("ERROR".to_string(), msg, Color::Red),
        Level::Warn => ("WARNING".to_string(), msg, Color::Yellow),
        Level::Debug => ("DEBUG".to_string(), msg, Color::Yellow),
        _ => {
            let split = msg.find(" ").unwrap();
            let first_word = msg[..split].to_string();
            let rest = msg[split+1..].to_string();
            let color = match first_word.as_str() {
                "Searching" | "Downloading" | "Loading" => Color::Blue,
                "Found" | "Saved" => Color::Green,
                _ => Color::BrightYellow,
            };
            (first_word, rest, color)
        }
    }
}


pub fn error(msg: &str) {
    println!("{:>12} {}", "Error".bold().red(), msg);
}
