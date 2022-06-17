use log::{Level, LevelFilter, Metadata};
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
                rest,
            ))
        })
        .level(level)
        .filter(|metadata| metadata.level() != Level::Debug || filter_log_message(metadata))
        .chain(std::io::stderr())
        .apply()?;
    Ok(())
}


fn format_log_message(msg: String, level: Level) -> (String, String, Color) {
    match level {
        Level::Error => ("ERROR".to_string(), msg, Color::Red),
        Level::Warn => ("WARNING".to_string(), msg, Color::Yellow),
        Level::Debug => ("DEBUG".to_string(), msg, Color::Yellow),
        _ => {
            let split = msg.find(" ").unwrap();
            let first_word = msg[..split].to_string();
            let rest = msg[split+1..].to_string();
            let color = match first_word.as_str() {
                "Searching" | "Downloading" | "Loading" | "Skipping" => Color::Blue,
                "Found" | "Saved" | "Added" => Color::Green,
                _ => Color::BrightYellow,
            };
            (first_word, rest, color)
        }
    }
}


/// Filter out log messages based on target
fn filter_log_message(metadata: &Metadata) -> bool {
    ![
        "selectors::matching",
        "html5ever::tokenizer",
        "html5ever::tokenizer::char_ref",
        "html5ever::tree_builder",
    ].contains(&metadata.target())
}


/// Prints a comic book to stdout
pub fn print_comic(comic: &grawlix::comic::Comic, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(comic).unwrap());
    } else {
        println!("{}", comic.title().bold());
        let metadata = &comic.metadata;
        let data = [
            ("Series", &metadata.series),
            ("Relase date", &metadata.date()),
            ("Publisher", &metadata.publisher),
            ("Pages", &Some(comic.pages.len().to_string())),
        ];
        for (name, opt_value) in data {
            if let Some(value) = opt_value {
                println!("{}: {}", name, value);
            }
        }
    }
}
