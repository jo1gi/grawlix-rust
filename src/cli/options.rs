use std::path::PathBuf;
use structopt::StructOpt;
use serde::Deserialize;
use grawlix::source::Credentials;

/// Command line comic book tool
#[derive(StructOpt)]
pub struct Arguments {
    /// Output template
    #[structopt(short, long, global = true)]
    pub output_template: Option<String>,
    /// Logging level
    #[structopt(short, long, default_value="info", global = true)]
    pub log_level: log::LevelFilter,
    /// Output format (Either cbz or dir)
    #[structopt(long, global = true)]
    pub output_format: Option<grawlix::comic::ComicFormat>,
    /// Overwrite already existing files
    #[structopt(long, global = true)]
    pub overwrite: bool,
    /// Path of file containing input urls
    #[structopt(short, long, global = true)]
    pub file: Option<PathBuf>,
    /// Save progress when pressing ctrl-c and continue download if progress file exists
    #[structopt(long, name = "continue", global = true)]
    pub use_progress_file: bool,
    /// Print extra information to stdout
    #[structopt(long, global = true)]
    pub info: bool,
    /// Output as json
    #[structopt(long, global = true)]
    json: bool,
    /// Location of update file to use
    #[structopt(long, global = true)]
    pub update_location: Option<String>,
    /// Subcommand
    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(StructOpt)]
pub enum Command {
    /// Add to update file
    Add {
        /// Links to comic books
        inputs: Vec<String>,
    },
    /// Download comics
    Download {
        /// Link to comic book
        inputs: Vec<String>,
    },
    /// Print comic metadata to stdout
    Info {
        /// Link to comic book
        inputs: Vec<String>,
    },
    /// List all series added to updatefile
    List,
    /// Update comics in updatefile
    Update
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
    /// Print output as json
    #[serde(default = "Default::default")]
    pub json: bool,
    /// Update file
    #[serde(default = "default_update")]
    pub update_location: String,
    /// DC Universe Infinite Config
    #[serde(default = "Default::default")]
    pub dcuniverseinfinite: Option<SourceData>,
    /// Marvel Config
    #[serde(default = "Default::default")]
    pub marvel: Option<SourceData>,
}

#[derive(Deserialize, Debug, Clone)]
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
        } else if self.username.is_some() && self.password.is_some() {
            Ok(Credentials::UsernamePassword(self.username.unwrap().clone(), self.password.unwrap().clone()))
        } else {
            Err(crate::CliError::InvalidCredentials)
        }
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
        output_format,
        update_location
    );
    args_into_config_bool!(args, config,
        overwrite,
        use_progress_file,
        info,
        json
    );
    return Some(config);
}

fn default_template() -> String {
    String::from("{series}/{title}.cbz")
}

fn default_update() -> String {
    String::from("./.grawlix-update")
}
