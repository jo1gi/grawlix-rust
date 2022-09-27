use std::collections::HashMap;

use crate::{
    comic::Page, metadata::{Author, AuthorType, Metadata},
    source::{
        ComicId, Error, Request, Result, Source, SourceResponse, SeriesInfo,
        utils::{
            source_request, first_text, first_attr, issue_id_match, value_fn,
            simple_response, ANDROID_USER_AGENT
        }
    }};
use reqwest::{Client, header};
use scraper::{Html, Selector};

pub struct Webtoon;

fn id_from_url(url: &str) -> Result<ComicId> {
    issue_id_match!(url,
        r"(\w+/[^/]+/[^/]+/viewer\?.+episode_no=\d+)" => Issue,
        r"(\w+/[^/]+/list\?title_no=\d+)" => Series
    )
}

impl Source for Webtoon {
    fn name(&self) -> String {
        "Webtoon".to_string()
    }

    fn create_client(&self) -> Client {
        let mut headers = header::HeaderMap::new();
        headers.insert("Cookie", header::HeaderValue::from_static("needGDPR=false; needCCPA=false; needCOPPA=false"));
        Client::builder()
            .default_headers(headers)
            .build()
            .unwrap()
    }

    fn id_from_url(&self, url: &str) -> Result<ComicId> {
        id_from_url(url)
    }

    fn get_series_ids(&self, client: &Client, seriesid: &ComicId) -> Result<Request<Vec<ComicId>>> {
        if let ComicId::Series(x) = seriesid {
            source_request!(
                requests:
                    client.get(format!("https://m.webtoons.com/en/{}", x))
                        .header("User-Agent", ANDROID_USER_AGENT),
                transform: |resp| {
                    let html = std::str::from_utf8(&resp[0]).ok()?;
                    let doc = Html::parse_document(html);
                    let issues = doc.select(&Selector::parse("ul#_episodeList").unwrap()).next()?;
                    issues.select(&Selector::parse("li a").unwrap())
                        .map(|issue| {
                            let link = issue.value().attr("href")?;
                            id_from_url(link).ok()
                        })
                        .collect()
                }
            )
        } else { Err(Error::FailedResponseParse) }
    }

    fn get_series_info(&self, client: &Client, seriesid: &ComicId) -> Result<SourceResponse<SeriesInfo>> {
        if let ComicId::Series(x) = seriesid {
            Ok(SourceResponse::Request(source_request!(
                requests:
                    client.get(format!("https://m.webtoons.com/en/{}", x))
                        .header("User-Agent", ANDROID_USER_AGENT),
                transform: value_fn(&response_series_info)
            ).unwrap()))
        } else { Err(Error::FailedResponseParse) }
    }

    fn get_metadata(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Metadata>> {
        simple_response!(
            id: comicid,
            client: client,
            id_type: Issue,
            url: "https://www.webtoons.com/en/{}",
            value: parse_metadata
        )
    }

    fn get_pages(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Vec<Page>>> {
        simple_response!(
            id: comicid,
            client: client,
            id_type: Issue,
            url: "https://www.webtoons.com/en/{}",
            value: response_to_pages
        )
    }
}

fn response_series_info(resp: &[bytes::Bytes]) -> Option<SeriesInfo> {
    let html = std::str::from_utf8(&resp[0]).ok()?;
    let doc = Html::parse_document(html);
    Some(SeriesInfo{
        name: first_attr(&doc, r#"meta[property="og:title"]"#, "content")?
    })
}

fn parse_metadata(resp: &[bytes::Bytes]) -> Option<Metadata> {
    let html = std::str::from_utf8(&resp[0]).ok()?;
    let doc = Html::parse_document(html);
    Some(Metadata {
        title: first_text(&doc, ".subj_episode"),
        series: first_text(&doc, ".subj"),
        authors: vec![find_author(&doc)?],
        description: first_attr(&doc, r#"meta[property="og:description"]"#, "content"),
        source: Some("Webtoon".to_string()),
        ..Default::default()
    })
}

fn find_author(doc: &Html) -> Option<Author> {
    Some(Author {
        name: doc.select(&Selector::parse(r#"meta[property="com-linewebtoon:episode:author"]"#).unwrap())
            .next()?
            .value()
            .attr("content")?
            .to_string(),
        author_type: AuthorType::Writer
    })
}

fn response_to_pages(resp: &[bytes::Bytes]) -> Option<Vec<Page>> {
    let html = std::str::from_utf8(&resp[0]).ok()?;
    let doc = Html::parse_document(html);
    let headers = HashMap::from([("Referer".to_string(), "www.webtoons.com".to_string())]);
    let images = doc.select(&Selector::parse("#content ._images").unwrap())
        .map(|element| {
            let url = element.value().attr("data-url")?;
            Some(Page::from_url_with_headers(&url, headers.clone(), "jpg"))
        })
        .collect();
    images
}

#[cfg(test)]
mod tests {
    use crate::{metadata::Author, source::{ComicId, Source, utils::tests::response_from_testfile}};

    #[test]
    fn issueid_from_url() {
        let source = super::Webtoon;
        assert_eq!(
            source.id_from_url("https://www.webtoons.com/en/challenge/the-weekly-roll/ch-116-grimdahls-folly/viewer?title_no=358889&episode_no=118").unwrap(),
            ComicId::Issue("challenge/the-weekly-roll/ch-116-grimdahls-folly/viewer?title_no=358889&episode_no=118".to_string())
        );
    }

    #[test]
    fn seriesid_from_url() {
        let source = super::Webtoon;
        assert_eq!(
            source.id_from_url("https://www.webtoons.com/en/challenge/the-weekly-roll/list?title_no=358889").unwrap(),
            ComicId::Series("challenge/the-weekly-roll/list?title_no=358889".to_string())
        );
    }

    #[test]
    fn series() {
        let source = super::Webtoon;
        let series_id = source.id_from_url("https://www.webtoons.com/en/challenge/the-weekly-roll/list?title_no=358889")
            .unwrap();
        let client = source.create_client();
        let parser = source.get_series_ids(&client, &series_id).unwrap().transform;
        let responses = response_from_testfile("webtoon_series.html");
        let issues = parser(&responses).unwrap();
        assert_eq!(issues.len(), 116);
        let info = super::response_series_info(&responses).unwrap();
        assert_eq!(info.name, "The Weekly Roll".to_string());
    }

    #[test]
    fn get_correct_number_of_pages() {
        let responses = response_from_testfile("webtoon_issue.html");
        let pages = super::response_to_pages(&responses).unwrap();
        assert_eq!(pages.len(), 6);
    }

    #[test]
    fn metadata() {
        let responses = std::fs::read("./tests/source_data/webtoon_issue.html").unwrap();
        let metadata = super::parse_metadata(&[responses.into()]).unwrap();
        assert_eq!(
            metadata,
            crate::metadata::Metadata {
                title: Some("Ch. 1. The lost virtue of de-escalation".to_string()),
                series: Some("The Weekly Roll".to_string()),
                authors: vec![
                    Author { name: "CME_T".to_string(), author_type: crate::metadata::AuthorType::Writer }
                ],
                description: Some("A weekly four-panel comic strip that follows the exploits of a party of adventurers as they walk the fine line between being the good guys and homeless psychopaths for hire. \n\nUpdates every Weekend".to_string()),
                source: Some("Webtoon".to_string()),
                ..Default::default()
            }
        );
    }
}
