mod dcuniverseinfinite;
mod flipp;
mod izneo;
mod leagueoflegends;
mod mangaplus;
mod marvel;
mod webtoon;

pub use dcuniverseinfinite::DCUniverseInfinite;
pub use flipp::Flipp;
pub use leagueoflegends::LeagueOfLegends;
pub use mangaplus::MangaPlus;
pub use marvel::Marvel;
pub use webtoon::Webtoon;

use crate::{
    error::GrawlixDownloadError as Error,
    source::{Source, Result},
};

/// Find first matching regular expression and evaluated corresponding expression
macro_rules! match_re {
    ($url:expr, $($pattern:expr => $e:expr),+) => (
        $(
            let re = regex::Regex::new($pattern).unwrap();
            if re.is_match($url) {
                return Ok(Box::new($e));
            }
        )+
    )
}

/// Create a corresponding `Source` trait object from url
pub fn source_from_url(url: &str) -> Result<Box<dyn Source>> {
    match_re!(url,
        "dcuniverseinfinite.com" => dcuniverseinfinite::DCUniverseInfinite::default(),
        "flipp.dk" => flipp::Flipp,
        "izneo.com" => izneo::Izneo,
        "universe.leagueoflegends.com" => leagueoflegends::LeagueOfLegends,
        "mangaplus.shueisha.co.jp" => mangaplus::MangaPlus,
        "marvel.com" => marvel::Marvel,
        "webtoons.com" => webtoon::Webtoon
    );
    Err(Error::UrlNotSupported(url.to_string()))
}

/// Create source object from name
pub fn source_from_name(name: &str) -> Result<Box<dyn Source>> {
    let lower = name.to_lowercase();
    Ok(match lower.as_str() {
        "dc" | "dcuniverseinfinite" => Box::new(dcuniverseinfinite::DCUniverseInfinite::default()),
        "flipp" => Box::new(flipp::Flipp),
        "izneo" => Box::new(izneo::Izneo),
        "league of legends" => Box::new(leagueoflegends::LeagueOfLegends),
        "manga plus" => Box::new(mangaplus::MangaPlus),
        "marvel" => Box::new(marvel::Marvel),
        "webtoon" => Box::new(webtoon::Webtoon),
        _ => return Err(Error::InvalidSourceName(name.to_string()))
    })
}
