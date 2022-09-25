use crate::{
    source::{
        Source, ComicId, Result, Request, SourceResponse, SeriesInfo, Credentials,
        utils::{
            issue_id_match, source_request, first_capture, value_to_optstring, resp_to_json,
            simple_request, simple_response
        },
    },
    metadata::{self, Metadata, Author},
    comic::Page,
};

use regex::Regex;
use reqwest::{
    Client,
    header::{HeaderValue, HeaderMap}
};

pub struct Marvel;

#[async_trait::async_trait]
impl Source for Marvel {
    fn name(&self) -> String {
        "Marvel".to_string()
    }

    fn create_client(&self) -> reqwest::Client {
        reqwest::ClientBuilder::new()
            .cookie_store(true)
            .build()
            .unwrap()
    }

    fn id_from_url(&self, url: &str) -> Result<ComicId> {
        issue_id_match!(url,
            r"issue/(\d+)" => Other,
            r"series/(\d+)" => Series
        )
    }

    fn get_correct_id(&self, client: &Client, otherid: &ComicId) -> Result<Request<ComicId>> {
        simple_request!(
            id: otherid,
            client: client,
            id_type: Other,
            url: "https://www.marvel.com/secure/purchasepathwidget/{}",
            transform: find_correct_id
        )
    }

    fn get_series_ids(&self, client: &Client, seriesid: &ComicId) -> Result<Request<Vec<ComicId>>> {
        simple_request!(
            id: seriesid,
            client: client,
            id_type: Series,
            url: "https://api.marvel.com/browse/comics?byType=comic_series&isDigital=1&limit=10000&byId={}",
            transform: find_series_ids
        )
    }

    fn get_series_info(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<SeriesInfo>> {
        simple_response!(
            id: comicid,
            client: client,
            id_type: Series,
            url: "https://api.marvel.com/browse/comics?byType=comic_series&isDigital=1&limit=10000&byId={}",
            transform: find_series_info
        )
    }

    fn get_pages(&self, client: &Client, comicid: &ComicId) -> Result<Request<Vec<Page>>> {
        simple_request!(
            id: comicid,
            client: client,
            id_type: Issue,
            url: "https://bifrost.marvel.com/v1/catalog/digital-comics/web/assets/{}",
            transform: find_pages
        )
    }

    fn get_metadata(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Metadata>> {
        simple_response!(
            id: comicid,
            client: client,
            id_type: Issue,
            url: "https://bifrost.marvel.com/v1/catalog/digital-comics/metadata/{}",
            transform: parse_metadata
        )
    }

    async fn authenticate(&mut self, client: &mut Client, creds: &Credentials) -> Result<()> {
        if let Credentials::UsernamePassword(username, password) = creds {
            let mut headers = HeaderMap::new();
            headers.insert("User-Agent", HeaderValue::from_static("aXMLRPC"));
            headers.insert("Content-Type", HeaderValue::from_static("text/html; charset=utf-8"));
            client.post("https://api.marvel.com/xmlrpc/login_api_https.php")
                .headers(headers)
                .body(format!(
                    r#"
                    <?xml version="1.0" encoding="UTF-8"?>
                    <methodCall>
                        <methodName>login</methodName>
                        <params>
                            <param><value><string>{username}</string></value></param>
                            <param><value><string>{password}</string></value></param>
                        </params>
                    </methodCall>
                   "#,
                    username=username,
                    password=password
                ))
                .send()
                .await?;
            // TODO: Check valid login
            Ok(())
        } else {
            unreachable!()
        }
    }
}

fn find_correct_id(resp: &[bytes::Bytes]) -> Option<ComicId> {
    let data = std::str::from_utf8(&resp[0]).ok()?;
    let re = Regex::new(r#"data-digitalid="(\d+)"#).unwrap();
    Some(ComicId::Issue(first_capture(&re, data)?))
}

fn find_series_ids(resp: &[bytes::Bytes]) -> Option<Vec<ComicId>> {
    Some(get_results(&resp[0])?
        .as_array()?
        .iter()
        .filter_map(|x| {
            Some(ComicId::Issue(value_to_optstring(&x["digital_id"])?))
        })
        .collect()
    )
}

fn find_series_info(resp: &[bytes::Bytes]) -> Option<SeriesInfo> {
    todo!()
}

fn find_pages(resp: &[bytes::Bytes]) -> Option<Vec<Page>> {
    Some(get_results(&resp[0])?[0]["pages"]
        .as_array()?
        .iter()
        .filter_map(|x| {
            Some(Page::from_url(&value_to_optstring(&x["assets"]["source"])?, "jpg"))
        })
        .collect())
}

/// Parse metadata from Marvel Unlimited issue
fn parse_metadata(responses: &[bytes::Bytes]) -> Option<Metadata> {
    let results = get_results(&responses[0])?;
    let issue_meta = &results[0]["issue_meta"];
    let date = metadata::date_from_str(&issue_meta["release_date"].as_str()?)?;
    Some(Metadata{
        title: value_to_optstring(&issue_meta["title"]),
        series: value_to_optstring(&issue_meta["series_title"]),
        publisher: Some("Marvel".to_string()),
        year: Some(date.0),
        month: Some(date.1),
        day: Some(date.2),
        authors: issue_meta["creators"]["extended_list"]
            .as_array()?
            .iter()
            .filter_map(|x| {
                Some(Author {
                    name: value_to_optstring(&x["full_name"])?,
                    author_type: value_to_optstring(&x["role"])?.into()
                })
            })
            .collect(),
        ..Default::default()
    })
}

/// Converts response to json and extracts results
fn get_results(response: &bytes::Bytes) -> Option<serde_json::Value> {
    let root: serde_json::Value = resp_to_json(response)?;
    let results = &root["data"]["results"];
    return Some(results.clone());
}

#[cfg(test)]
mod tests {

    use crate::source::{Source, ComicId};
    use crate::metadata::{Author, AuthorType, Metadata};

    #[test]
    fn series_ids() {
        let data = std::fs::read("./tests/source_data/marvel_series.json").unwrap();
        let responses = [data.into()];
        let ids = super::find_series_ids(&responses).unwrap();
        println!("{:#?}", ids);
        assert_eq!(ids.len(), 22);
    }

    #[test]
    fn pages() {
        let data = std::fs::read("./tests/source_data/marvel_pages.json").unwrap();
        let responses = [data.into()];
        let pages = super::find_pages(&responses).unwrap();
        assert_eq!(pages.len(), 3);
    }

    #[test]
    fn id() {
        let source = super::Marvel;
        assert_eq!(
            source.id_from_url("https://www.marvel.com/comics/issue/42768/hawkeye_2012_1").unwrap(),
            ComicId::Other("42768".to_string())
        );
        assert_eq!(
            source.id_from_url("https://www.marvel.com/comics/series/16309/hawkeye_2012_-_2015").unwrap(),
            ComicId::Series("16309".to_string())
        );
        let data = std::fs::read("./tests/source_data/marvel_issue.html").unwrap();
        let responses = [data.into()];
        assert_eq!(
            super::find_correct_id(&responses),
            Some(ComicId::Issue("28403".to_string()))
        );
    }

    #[test]
    fn metadata() {
        let data = std::fs::read("./tests/source_data/marvel_issue.json").unwrap();
        let responses = [data.into()];
        assert_eq!(
            super::parse_metadata(&responses).unwrap(),
            Metadata {
                title: Some("Hawkeye (2012) #7".to_string()),
                series: Some("Hawkeye (2012 - 2015)".to_string()),
                publisher: Some("Marvel".to_string()),
                year: Some(2013),
                month: Some(1),
                day: Some(30),
                authors: vec![
                    Author { name: "Matt Fraction".to_string(), author_type: AuthorType::Writer },
                    Author { name: "Steve Lieber".to_string(), author_type: AuthorType::Inker },
                    Author { name: "Jesse Alan Hamm".to_string(), author_type: AuthorType::Inker },
                    Author { name: "Matt Hollingsworth".to_string(), author_type: AuthorType::Colorist },
                    Author { name: "David Aja".to_string(), author_type: AuthorType::CoverArtist },
                    Author { name: "Virtual Calligr".to_string(), author_type: AuthorType::Letterer },
                    Author { name: "Stephen Wacker".to_string(), author_type: AuthorType::Editor },
                ],
                ..Default::default()
            }
        );
    }
}
