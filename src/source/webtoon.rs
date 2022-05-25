use std::collections::HashMap;

use crate::{
    comic::Page,
    metadata::Metadata,
    source::{
        ComicId, Error, Request, Result, Source, SourceResponse,
        issue_id_match,
        tools::{source_request, first_text, ANDROID_USER_AGENT}
    }
};
use reqwest::{Client, header};
use scraper::{Html, Selector};

pub struct Webtoon;

fn id_from_url(url: &str) -> Result<ComicId> {
    issue_id_match!(url,
        r"(challenge/.+/viewer\?.+episode_no=\d+)" => Issue,
        r"(challenge/.+/list\?title_no=\d+)" => Series
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

    fn get_metadata(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Metadata>> {
        if let ComicId::Issue(x) = comicid {
            Ok(SourceResponse::Request(source_request!(
                requests: client.get(format!("https://www.webtoons.com/en/{}", x)),
                transform: parse_metadata
            ).unwrap()))
        } else { Err(Error::FailedResponseParse) }
    }

    fn get_pages(&self, client: &Client, comicid: &ComicId) -> Result<Request<Vec<Page>>> {
        if let ComicId::Issue(x) = comicid {
            source_request!(
                requests: client.get(format!("https://www.webtoons.com/en/{}", x)),
                transform: response_to_pages
            )
        } else {Err(Error::FailedResponseParse)}
    }
}

fn parse_metadata(resp: &[bytes::Bytes]) -> Option<Metadata> {
    let html = std::str::from_utf8(&resp[0]).ok()?;
    let doc = Html::parse_document(html);
    Some(Metadata {
        title: first_text(&doc, ".subj_episode"),
        series: first_text(&doc, ".subj"),
        ..Default::default()
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
    use crate::source::{ComicId, Source};

    #[test]
    fn issue_id() {
        let source = super::Webtoon;
        assert_eq!(
            source.id_from_url("https://www.webtoons.com/en/challenge/the-weekly-roll/ch-116-grimdahls-folly/viewer?title_no=358889&episode_no=118").unwrap(),
            ComicId::Issue("challenge/the-weekly-roll/ch-116-grimdahls-folly/viewer?title_no=358889&episode_no=118".to_string())
        );
        assert_eq!(
            source.id_from_url("https://www.webtoons.com/en/challenge/the-weekly-roll/list?title_no=358889").unwrap(),
            ComicId::Series("challenge/the-weekly-roll/list?title_no=358889".to_string())
        );
    }

    #[test]
    fn series() {
        let source = super::Webtoon;
        let series_id = source.id_from_url("https://www.webtoons.com/en/challenge/the-weekly-roll/list?title_no=358889").unwrap();
        let client = source.create_client();
        let parser = source.get_series_ids(&client, &series_id).unwrap().transform;
        let response = std::fs::read("./tests/source_data/webtoon_series.html").unwrap();
        let series = parser(&[response.into()]).unwrap();
        assert_eq!(series.len(), 116);
    }

    #[test]
    fn pages() {
        let responses = std::fs::read("./tests/source_data/webtoon_issue.html").unwrap();
        let pages = super::response_to_pages(&[responses.into()]).unwrap();
        assert_eq!(pages.len(), 6);
    }
}
