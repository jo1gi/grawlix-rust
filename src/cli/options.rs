use std::path::PathBuf;
use structopt::StructOpt;
use serde::Deserialize;
use grawlix::source::Credentials;

/// Command line comic book tool
#[derive(StructOpt, Debug)]
pub struct Arguments {
    /// Path or link to comic book
    pub inputs: Vec<String>,
    /// Output template
    #[structopt(short, long)]
    pub output_template: Option<String>,
    /// Logging level
    #[structopt(short, long, default_value="info")]
    pub log_level: log::LevelFilter,
    /// Output format (Either cbz or dir)
    #[structopt(long)]
    pub output_format: Option<grawlix::comic::ComicFormat>,
    /// Overwrite already existing files
    #[structopt(long)]
    pub overwrite: bool,
    /// Path of file containing input urls
    #[structopt(short, long)]
    pub file: Option<PathBuf>,
    /// Save progress when pressing ctrl-c and continue download if progress file exists
    #[structopt(long, name = "continue")]
    pub use_progress_file: bool,
    /// Print extra information to stdout
    #[structopt(long)]
    pub info: bool,
}


#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    /// Template for output locations of comics
    #[serde(rename = "template", default = "default_template")]
    pub output_template: String,
    /// File format for output comics
    #[serde(default = "Default::default")]
    pub output_format: grawlix::comic::ComicFormat,
    /// Should overwrite already existing files if enabled
    #[serde(default = "Default::default")]
    pub overwrite: bool,
    /// Save progress when pressing ctrl-c and continue download if progress file exists
    #[serde(rename = "continue", default = "Default::default")]
    pub use_progress_file: bool,
    /// Print extra information to stdout
    #[serde(default = "Default::default")]
    pub info: bool,
}

#[derive(Deserialize, Debug)]
pub struct SourceData {
    pub username: Option<String>,
    pub password: Option<String>,
    pub api_key: Option<String>,
}

impl TryInto<Credentials> for SourceData {
    type Error = crate::CliError;

    fn try_into(self) -> Result<Credentials, Self::Error> {
        if let Some(api_key) = self.api_key {
            Ok(Credentials::ApiKey(api_key))
        } else { Err(crate::CliError::InvalidCredentials) }
    }
}

/// Loads config file if it exists
fn load_config_from_file() -> Option<Config> {
    let config_path = dirs::config_dir()?.as_path().join("grawlix/grawlix.toml");
    let config = if config_path.exists() {
        std::fs::read_to_string(config_path).ok()?
    } else {
        String::from("")
    };
    toml::from_str(&config).ok()
}

macro_rules! args_into_config_opt {
    ($args:expr, $config:expr, $($path:ident),+) => (
        $(
            match &$args.$path {
                Some(x) => $config.$path = x.clone(),
                None => ()
            }
        )+
    )
}

macro_rules! args_into_config_bool {
    ($args:expr, $config:expr, $($path:ident),+) => (
        $(
            if $args.$path {
                $config.$path = true;
            }
        )+
    )
}

/// Loads options from config file and command line arguments
pub fn load_options(args: &Arguments) -> Option<Config> {
    let mut config = load_config_from_file()?;
    args_into_config_opt!(args, config,
        output_template,
        output_format
    );
    args_into_config_bool!(args, config,
        overwrite,
        use_progress_file,
        info
    );
    return Some(config);
}

fn default_template() -> String {
    String::from("{series}/{title}")
}
