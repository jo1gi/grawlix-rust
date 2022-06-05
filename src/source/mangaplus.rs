use regex::bytes::Regex;

use crate::{
    source::{
        Source, ComicId, Result, Error, Request, SourceResponse,
        utils::{issue_id_match, source_request}
    },
    metadata::Metadata,
    comic::Page,
};
use reqwest::Client;

pub struct MangaPlus;

impl Source for MangaPlus {
    fn name(&self) -> String {
        "Manga Plus".to_string()
    }

    fn id_from_url(&self, url: &str) -> Result<ComicId> {
        issue_id_match!(url,
            r"viewer/(\d+)" => Issue
        )
    }

    fn get_series_ids(&self, client: &Client, seriesid: &ComicId) -> Result<Request<Vec<ComicId>>> {
        todo!()
    }

    fn get_metadata(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Metadata>> {
        if let ComicId::Issue(x) = comicid {
            Ok(SourceResponse::Value(Metadata {
                ..Default::default()
            }))
        } else { Err(Error::FailedResponseParse) }
    }

    fn get_pages(&self, client: &Client, comicid: &ComicId) -> Result<Request<Vec<Page>>> {
        if let ComicId::Issue(x) = comicid {
            source_request!(
                requests: client.get(
                    format!("https://jumpg-webapi.tokyo-cdn.com/api/manga_viewer?chapter_id={}&split=yes&img_quality=super_high", x)
                ),
                transform: response_to_pages
            )
        } else { Err(Error::FailedResponseParse) }
    }
}

fn response_to_pages(resp: &[bytes::Bytes]) -> Option<Vec<Page>> {
    // let url_regex = Regex::new(r"(?s:\x01(?P<url>.+)\x10.+\x01(?P<key>.{128})\x0a)").unwrap();
    let key_regex = Regex::new(r"\x01(?P<key>.{128})\x0a").unwrap();
    let url_regex = Regex::new(r"\x01(?P<url>.+)\x10").unwrap();
    url_regex.captures_iter(&resp[0])
        .zip(key_regex.captures_iter(&resp[0]))
        .map(|(url, key)| {
            let url = std::str::from_utf8(&url["url"]).ok()?;
            let hex_key = std::str::from_utf8(&key["key"]).ok()?;
            let key = hex_to_bin(hex_key)?;
            Some(Page::from_url_xor(url, key, "jpg"))
        })
        .collect()
}

/// Converts a hex number to a `Vec<u8>` by splitting them up in pairs of 2 and converting
fn hex_to_bin(hex: &str) -> Option<Vec<u8>> {
    (0..hex.len()).step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i+2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {

    const HEXKEY: &str = "47ccd43a81558cfbd272a5d04d6233ad7cd56f790285f239103d0b6dd887959aff344ce7089a508d1650e6b45626934e528e61f5fbe17236efd2567543bb0c51";

    #[test]
    fn hex_to_bin() {
        assert_eq!(
            super::hex_to_bin(HEXKEY).unwrap(),
            vec![71, 204, 212, 58, 129, 85, 140, 251, 210, 114, 165, 208, 77, 98, 51, 173, 124, 213, 111, 121, 2, 133, 242, 57, 16, 61, 11, 109, 216, 135, 149, 154, 255, 52, 76, 231, 8, 154, 80, 141, 22, 80, 230, 180, 86, 38, 147, 78, 82, 142, 97, 245, 251, 225, 114, 54, 239, 210, 86, 117, 67, 187, 12, 81]
        );
    }

    #[test]
    fn pages() {
        let responses = std::fs::read("./tests/source_data/mangaplus_issue").unwrap();
        let pages = super::response_to_pages(&[responses.into()]).unwrap();
        assert_eq!(pages.len(), 53);
    }
}
