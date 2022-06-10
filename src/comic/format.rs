use super::Comic;
use rt_format::{Format, FormatArgument, ParsedFormat, Specifier};
use std::collections::HashMap;
use std::fmt;
use crate::metadata::{Author, AuthorType};

#[derive(Debug, PartialEq, Clone)]
pub enum Variant {
    String(String),
    Int(u32),
}

impl Variant {
    fn string(s: &Option<String>) -> Option<Self> {
        s.as_ref().map(|x| Self::String(x.clone()))
    }

    fn int(s: &Option<u32>) -> Option<Self> {
        s.as_ref().map(|x| Self::Int(*x))
    }
}

impl FormatArgument for Variant {
    fn supports_format(&self, spec: &Specifier) -> bool {
        match self {
            Self::Int(_) => true,
            Self::String(_) => match spec.format {
                Format::Display | Format::Debug => true,
                _ => false
            },
        }
    }

    fn fmt_display(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Int(val) => fmt::Display::fmt(&val, f),
            Self::String(val) => fmt::Display::fmt(&val, f),
        }
    }

    fn fmt_debug(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }

    fn fmt_octal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Int(val) => fmt::Octal::fmt(&val, f),
            _ => Err(fmt::Error),
        }
    }

    fn fmt_lower_hex(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Int(val) => fmt::LowerHex::fmt(&val, f),
            _ => Err(fmt::Error),
        }
    }
 
    fn fmt_upper_hex(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Int(val) => fmt::UpperHex::fmt(&val, f),
            _ => Err(fmt::Error),
        }
    }
 
    fn fmt_binary(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Int(val) => fmt::Binary::fmt(&val, f),
            _ => Err(fmt::Error),
        }
    }
 
    fn fmt_lower_exp(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Int(val) => fmt::LowerExp::fmt(&val, f),
            _ => Err(fmt::Error)
        }
    }
 
    fn fmt_upper_exp(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Int(val) => fmt::UpperExp::fmt(&val, f),
            _ => Err(fmt::Error)
        }
    }

     fn to_usize(&self) -> Result<usize, ()> {
        match self {
            Variant::Int(val) => (*val).try_into().map_err(|_| ()),
            _ => Err(()),
        }
    }
}

fn get_first_author(authors: &Vec<Author>, author_type: AuthorType) -> Option<String> {
    authors.iter()
        .find(|x| x.author_type == author_type)
        .map(|x| x.name.clone())
}

fn comic_options(comic: &Comic) -> HashMap<&str, Variant> {
    let meta = &comic.metadata;
    [
        ("title", Variant::string(&meta.title)),
        ("series", Variant::string(&meta.series)),
        ("publisher", Variant::string(&meta.publisher)),
        ("issuenumber", Variant::int(&meta.issue_number)),
        ("year", Variant::int(&meta.year)),
        ("month", Variant::int(&meta.month)),
        ("day", Variant::int(&meta.day)),
        ("writer", Variant::string(&get_first_author(&meta.authors, AuthorType::Writer))),
        ("penciller", Variant::string(&get_first_author(&meta.authors, AuthorType::Penciller))),
        ("inker", Variant::string(&get_first_author(&meta.authors, AuthorType::Inker))),
        ("colorist", Variant::string(&get_first_author(&meta.authors, AuthorType::Colorist))),
        ("letterer", Variant::string(&get_first_author(&meta.authors, AuthorType::Letterer))),
        ("coverartist", Variant::string(&get_first_author(&meta.authors, AuthorType::CoverArtist))),
        ("editor", Variant::string(&get_first_author(&meta.authors, AuthorType::Editor))),
        ("pages", Some(Variant::Int(comic.pages.len() as u32))),
    ].into_iter()
        .map(|(k, v)| (k, v.unwrap_or(Variant::String("Unknown".to_string()))))
        .collect()
}

impl Comic {
    /// Format comic as string based on metadata and template
    pub fn format(&self, template: &str) -> Result<String, crate::error::GrawlixIOError> {
        let named_options = comic_options(self);
        let args = ParsedFormat::parse(template, &[], &named_options)
            .map_err(|e| crate::error::GrawlixIOError::StringFormat(e, template.to_string()))?;
        return Ok(format!("{}", args));
    }
}

#[cfg(test)]
mod tests {
    use crate::comic::{Page, Comic};
    use crate::metadata::*;

    #[test]
    fn comic_formatting() {
        let mut comic = Comic::new();
        comic.pages = vec![ Page::from_url("link", "jpg") ];
        comic.metadata = Metadata {
            title: Some(String::from("Moon Knight #1")),
            series: Some(String::from("Moon Knight (2016 - 2018)")),
            publisher: Some(String::from("Marvel")),
            issue_number: Some(1),
            year: Some(2016),
            month: Some(4),
            day: Some(13),
            authors: vec![
                Author { name: "Jeff Lemire".to_string(), author_type: AuthorType::Writer },
                Author { name: "Greg Smallwood".to_string(), author_type: AuthorType::CoverArtist },
                Author { name: "Greg Smallwood".to_string(), author_type: AuthorType::Penciller },
            ],
            ..Default::default()
        };
        assert_eq!(
            "Marvel/Moon Knight (2016 - 2018)/Moon Knight (2016 - 2018) #1.cbz",
            comic.format("{publisher}/{series}/{series} #{issuenumber}.cbz").unwrap()
        );
        assert_eq!(
            "Moon Knight (2016 - 2018) by Jeff Lemire and Greg Smallwood",
            comic.format("{series} by {writer} and {penciller}").unwrap()
        );
        assert_eq!(
            "Moon Knight #1 Moon Knight (2016 - 2018) Marvel 1 2016 4 13 Jeff Lemire Greg Smallwood 1",
            comic.format("{title} {series} {publisher} {issuenumber} {year} {month} {day} {writer} {coverartist} {pages}").unwrap()
        );
    }
}
