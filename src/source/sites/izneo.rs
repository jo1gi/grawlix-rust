use reqwest::Client;

use crate::{
    comic::{Page, OnlinePage, PageEncryptionScheme, PageType},
    metadata::{Metadata, Author},
    source::{
        ComicId, Result, Source, SourceResponse, SeriesInfo,
        utils::{self, issue_id_match, simple_response, value_to_optstring}
    }
};

pub struct Izneo;

impl Source for Izneo {

    fn name(&self) -> String {
        "Izneo".to_string()
    }

    fn id_from_url(&self, url: &str) -> Result<ComicId> {
        id_from_url(url)
    }

    fn get_series_ids(&self, client: &Client, seriesid: &ComicId) -> Result<SourceResponse<Vec<ComicId>>>  {
        simple_response!(
            id: seriesid,
            client: client,
            id_type: Series,
            url: "https://izneo.com/en/api/android/serie/{}/volumes/old/0/10000",
            value: find_series_ids
        )
    }

    fn get_series_info(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<SeriesInfo>>  {
        simple_response!(
            id: comicid,
            client: client,
            id_type: Series,
            url: "https://izneo.com/en/api/android/serie/{}",
            value: find_series_info
        )
    }

    fn get_metadata(&self,client: &Client, comicid: &ComicId) -> Result<SourceResponse<Metadata>>  {
        simple_response!(
            id: comicid,
            client: client,
            id_type: Issue,
            url: "https://www.izneo.com/book/{}",
            value: parse_metadata
        )
    }

    fn get_pages(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Vec<Page>>> {
        simple_response!(
            id: comicid,
            client: client,
            id_type: Issue,
            url: "https://www.izneo.com/book/{}",
            value: get_pages
        )
    }

}

fn id_from_url(url: &str) -> Result<ComicId> {
    issue_id_match!(url,
        r"\w+/[^/]+/[^/]+/[^/]+/.+-(\d+)/read" => Issue,
        r"\w+/[^/]+/[^/]+/.+-(\d+)$" => Series
    )
}

fn find_series_info(resp: &[bytes::Bytes]) -> Option<SeriesInfo> {
    let root: serde_json::Value = utils::resp_to_json(&resp[0])?;
    Some(SeriesInfo {
        name: root["name"].as_str()?.to_string(),
        ..Default::default()
    })
}

fn find_series_ids(resp: &[bytes::Bytes]) -> Option<Vec<ComicId>> {
    let root: serde_json::Value = utils::resp_to_json(&resp[0])?;
    root["albums"]
        .as_array()?
        .iter()
        .map(|x| {
            let id = x["id"].as_str()?.to_string();
            Some(ComicId::Issue(id))
        })
        .collect()
}

fn get_pages(resp: &[bytes::Bytes]) -> Option<Vec<Page>> {
    let root: serde_json::Value = utils::resp_to_json(&resp[0])?;
    let data = &root["data"];
    let book = data["id"].as_str()?;
    let state = data["state"].as_str()?;
    let preview = if state == "preview" { "?type=preview" } else { "" };
    let pages = data["pages"]
        .as_array()?
        .iter()
        .filter_map(|x| {
            log::trace!("Page");
            let f = |v| {
                let string_value = value_to_optstring(v)?;
                base64::decode(&string_value).ok()
            };
            Some(Page {
                file_format: "jpg".to_string(),
                page_type: PageType::Url(OnlinePage {
                    url: format!(
                        "https://www.izneo.com/book/{book}/{page}{preview}",
                        book = book,
                        page = &x["albumPageNumber"].as_u64()?,
                        preview = preview
                    ),
                    headers: None,
                    encryption: Some(PageEncryptionScheme::AES {
                        key: f(&x["key"])?,
                        iv: f(&x["iv"])?,
                    })
                })
            })
        })
        .collect();
    Some(pages)
}

fn parse_metadata(resp: &[bytes::Bytes]) -> Option<Metadata> {
    let root: serde_json::Value = utils::resp_to_json(&resp[0])?;
    let data = &root["data"];
    let info = &data["endingPageRules"]["ctaAlbum"];
    Some(Metadata {
        title: value_to_optstring(&info["title"]),
        series: value_to_optstring(&info["serie_name"]),
        reading_direction: data["readDirection"].as_str()?.try_into().ok()?,
        authors: info["authors"]
            .as_array()?
            .iter()
            .filter_map(|author| Some(Author {
                name: author["nickname"].as_str()?.to_string(),
                author_type: crate::metadata::AuthorType::Other,
            }))
            .collect(),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use crate::metadata::{ReadingDirection, Author, AuthorType};
    use crate::source::ComicId;
    use crate::source::utils::tests as test_utils;


    #[test]
    fn issueid_from_url() {
        assert_eq!(
            super::id_from_url("https://www.izneo.com/en/us-comics/fantasy/jim-butcher-s-the-dresden-files-20229/jim-butcher-s-the-dresden-files-down-town-46333/read/1?exiturl=https://www.izneo.com/en/us-comics/fantasy/jim-butcher-s-the-dresden-files-20229").unwrap(),
            ComicId::Issue("46333".to_string())
        )
    }

    #[test]
    fn seriesid_from_url() {
        assert_eq!(
            super::id_from_url("https://www.izneo.com/en/us-comics/fantasy/jim-butcher-s-the-dresden-files-20229").unwrap(),
            ComicId::Series("20229".to_string())
        )
    }

    #[test]
    fn find_series_ids() {
        let responses = test_utils::response_from_testfile("izneo_series.json");
        let issues = super::find_series_ids(&responses).unwrap();
        assert_eq!(issues.len(), 7);
    }

    #[test]
    fn number_of_pages() {
        let responses = test_utils::response_from_testfile("izneo_issue.json");
        let pages = super::get_pages(&responses).unwrap();
        assert_eq!(pages.len(), 11);
    }

    #[test]
    fn metadata() {
        let response = test_utils::response_from_testfile("izneo_issue.json");
        assert_eq!(
            super::parse_metadata(&response).unwrap(),
            crate::metadata::Metadata {
                title: Some("Jim Butcher's The Dresden Files: Down Town".to_string()),
                series: Some("Jim Butcher's The Dresden Files".to_string()),
                reading_direction: ReadingDirection::LeftToRight,
                authors: vec![
                    Author { name: "Jim Butcher".to_string(), author_type: AuthorType::Other },
                    Author { name: "Mark Powers".to_string(), author_type: AuthorType::Other },
                ],
                ..Default::default()
            }
        )
    }

}
