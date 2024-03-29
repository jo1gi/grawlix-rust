use regex::bytes::Regex;
use reqwest::Client;

use crate::{
    comic::Page,
    metadata::{Metadata, ReadingDirection},
    source::{
        Source, ComicId, Result, SourceResponse, SeriesInfo,
        utils::{issue_id_match, first_capture_bin, simple_response}
    }
};


pub struct MangaPlus;

impl Source for MangaPlus {
    fn name(&self) -> String {
        "Manga Plus".to_string()
    }

    fn id_from_url(&self, url: &str) -> Result<ComicId> {
        issue_id_match!(url,
            r"viewer/(\d+)" => Issue,
            r"titles/(\d+)" => Series
        )
    }

    fn get_series_ids(&self, client: &Client, seriesid: &ComicId) -> Result<SourceResponse<Vec<ComicId>>> {
        simple_response!(
            id: seriesid,
            client: client,
            id_type: Series,
            url: "https://jumpg-api.tokyo-cdn.com/api/title_detailV2?title_id={}&lang=eng&os=android&os_ver=32&app_ver=40&secret=2afb69fbb05f57a1856cf75e1c4b6ee6",
            value: find_series_ids
        )
    }

    fn get_series_info(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<SeriesInfo>> {
        simple_response!(
            id: comicid,
            client: client,
            id_type: Series,
            url: "https://jumpg-webapi.tokyo-cdn.com/api/title_detailV2?title_id={}",
            value: response_series_info
        )
    }

    fn get_metadata(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Metadata>> {
        simple_response!(
            id: comicid,
            client: client,
            id_type: Issue,
            url: "https://jumpg-webapi.tokyo-cdn.com/api/manga_viewer?chapter_id={}&split=yes&img_quality=super_high",
            value: response_to_metadata
        )
    }

    fn get_pages(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Vec<Page>>> {
        simple_response!(
            id: comicid,
            client: client,
            id_type: Issue,
            url: "https://jumpg-webapi.tokyo-cdn.com/api/manga_viewer?chapter_id={}&split=yes&img_quality=super_high",
            value: response_to_pages
        )
    }
}

fn find_series_ids(resp: &[bytes::Bytes]) -> Option<Vec<ComicId>> {
    let url_re = Regex::new(r"chapter/(?P<id>(\d+))").unwrap();
    url_re.captures_iter(&resp[0])
        .map(|cap| {
            let id = std::str::from_utf8(&cap["id"]).ok()?.to_string();
            Some(ComicId::Issue(id))
        })
        .collect()
}

fn response_series_info(resp: &[bytes::Bytes]) -> Option<SeriesInfo> {
    let name_re = Regex::new(r#"(?s)\x12.(.+)\x1a"#).unwrap();
    Some(SeriesInfo {
        name: first_capture_bin(&name_re, &resp[0])?,
        ..Default::default()
    })
}

fn response_to_metadata(resp: &[bytes::Bytes]) -> Option<Metadata> {
    let title_re = Regex::new(r#"(?s)\x22.(.+)\x2a"#).unwrap();
    Some(Metadata {
        title: first_capture_bin(&title_re, &resp[0]),
        series: first_capture_bin(&Regex::new(r#"MANGA_Plus (.+)\x12"#).unwrap(), &resp[0]),
        reading_direction: ReadingDirection::RightToLeft,
        issue_number: first_capture_bin(&Regex::new(r#"#(\d+)"#).unwrap(), &resp[0])
            .map(|s| s.parse::<u32>().ok())
            .flatten(),
        source: Some("Manga Plus".to_string()),
        ..Default::default()
    })
}

fn response_to_pages(resp: &[bytes::Bytes]) -> Option<Vec<Page>> {
    // let url_regex = Regex::new(r"(?s:\x01(?P<url>.+)\x10.+\x01(?P<key>.{128})\x0a)").unwrap();
    let key_regex = Regex::new(r"\x01(?P<key>.{128})\x0a").unwrap();
    let url_regex = Regex::new(r"\x01(?P<url>https://mangaplus.shueisha.co.jp/drm/title/.+)\x10").unwrap();
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

    use crate::{
        metadata::ReadingDirection,
        source::{ComicId, Source, utils::tests::response_from_testfile}
    };


    const HEXKEY: &str = "47ccd43a81558cfbd272a5d04d6233ad7cd56f790285f239103d0b6dd887959aff344ce7089a508d1650e6b45626934e528e61f5fbe17236efd2567543bb0c51";

    #[test]
    fn issueid_from_url() {
        let source = super::MangaPlus;
        assert_eq!(
            source.id_from_url("https://mangaplus.shueisha.co.jp/viewer/1000486").unwrap(),
            ComicId::Issue("1000486".to_string())
        );
    }

    #[test]
    fn seriesid_from_url() {
        let source = super::MangaPlus;
        assert_eq!(
            source.id_from_url("https://mangaplus.shueisha.co.jp/titles/100020").unwrap(),
            ComicId::Series("100020".to_string())
        );
    }

    #[test]
    fn hex_to_bin() {
        assert_eq!(
            super::hex_to_bin(HEXKEY).unwrap(),
            vec![71, 204, 212, 58, 129, 85, 140, 251, 210, 114, 165, 208, 77, 98, 51, 173, 124, 213, 111, 121, 2, 133, 242, 57, 16, 61, 11, 109, 216, 135, 149, 154, 255, 52, 76, 231, 8, 154, 80, 141, 22, 80, 230, 180, 86, 38, 147, 78, 82, 142, 97, 245, 251, 225, 114, 54, 239, 210, 86, 117, 67, 187, 12, 81]
        );
    }

    #[test]
    fn number_of_pages() {
        let responses = response_from_testfile("mangaplus_issue");
        let pages = super::response_to_pages(&responses).unwrap();
        assert_eq!(pages.len(), 53);
    }

    #[test]
    fn metadata() {
        let responses = response_from_testfile("mangaplus_issue");
        let metadata = super::response_to_metadata(&responses).unwrap();
        assert_eq!(metadata, crate::metadata::Metadata {
            title: Some("Chapter 1: Romance Dawn".to_string()),
            series: Some("One Piece".to_string()),
            issue_number: Some(1),
            reading_direction: ReadingDirection::RightToLeft,
            source: Some("Manga Plus".to_string()),
            ..Default::default()
        });
    }

    #[test]
    fn parse_series_ids() {
        let responses = response_from_testfile("mangaplus_series");
        let issues = super::find_series_ids(&responses).unwrap();
        assert_eq!(issues.len(), 1051);
    }

    #[test]
    fn get_series_info() {
        let responses = response_from_testfile("mangaplus_series");
        let series_info = super::response_series_info(&responses).unwrap();
        assert_eq!(series_info.name, "One Piece".to_string());
    }
}
