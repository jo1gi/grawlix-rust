use std::collections::HashMap;

use crate::{
    source::{
        Source, SourceResponse, Request, Result, Error, ComicId, SeriesInfo,
        utils::{self, issue_id_match, resp_to_json, value_to_optstring, source_request, value_fn}
    },
    comic::Page,
    metadata::Metadata,
};
use regex::Regex;
use reqwest::Client;

pub struct Flipp;

impl Source for Flipp {
    fn name(&self) -> String {
        "Flipp".to_string()
    }

    fn id_from_url(&self, url: &str) -> Result<ComicId> {
        issue_id_match!(url,
            r"https?://reader.flipp.dk/html5/reader/production/default.aspx\?pubname=&edid=([^/]+)" => Other,
            r"^https?://magasiner.flipp.dk/flipp/web-app/#/publications/([^/]+)" => Series
        )
    }

    fn get_correct_id(&self, client: &Client, otherid: &ComicId) -> Result<Request<ComicId>> {
        if let ComicId::Other(eid) = otherid {
            let eid = eid.to_string();
            let url = format!(
                "https://reader.flipp.dk/html5/reader/production/default.aspx?pubname=&edid={}",
                eid
            );
            source_request!(
                requests: client.get(url),
                transform: move |resp| {
                    let site = std::str::from_utf8(&resp[0]).ok()?;
                    let pubid_re = Regex::new("(?:publicationguid = \")([^\"]+)").unwrap();
                    let pubid = pubid_re.captures(site)?.get(1)?.as_str().to_string();
                    return Some(ComicId::Issue(format!(
                        "https://reader.flipp.dk/html5/reader/get_page_groups_from_eid.aspx?pubid={}&eid={}",
                        pubid, eid
                    )));
                }
            )
        } else { Err(Error::FailedResponseParse) }
    }

    fn get_series_info(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<SeriesInfo>> {
        if let ComicId::Series(x) = comicid {
            let series_id = x.to_string();
            Ok(SourceResponse::Request(source_request!(
                requests: signin_data(client),
                transform: move |resp| {
                    let series_data = get_series_data(resp, &series_id)?;
                    Some(SourceResponse::Value(SeriesInfo {
                        name: series_data["name"].as_str()?.to_string()
                    }))
                }
            ).unwrap()))
        } else { Err(Error::FailedResponseParse) }
    }

    fn get_metadata(&self, _client: &Client, _comicid: &ComicId) -> Result<SourceResponse<Metadata>> {
        Ok(SourceResponse::Value(Metadata::default()))
    }

    fn get_series_ids(&self, client: &Client, seriesid: &ComicId) -> Result<Request<Vec<ComicId>>> {
        match seriesid {
            ComicId::Series(x) => {
                let series_id = x.to_string();
                source_request!(
                    requests: signin_data(client),
                    transform: move |resp| {
                        let series_data = get_series_data(resp, &series_id)?;
                        // Extracting issue data
                        let series_name = &series_data["name"].as_str()?;
                        let series_id = series_data["customPublicationCode"].as_str()?;
                        series_data["issues"]
                            .as_array()?
                            .iter()
                            .map(|issue| {
                                let issue_id = value_to_optstring(&issue["customIssueCode"])?;
                                let metadata = Metadata {
                                    title: Some(format!("{} {}", series_name, &issue["issueName"].as_str()?)),
                                    series: Some(series_name.to_string()),
                                    source: Some("Flipp".to_string()),
                                    ..Default::default()
                                };
                                let data_url = format!(
                                    "https://reader.flipp.dk/html5/reader/get_page_groups_from_eid.aspx?pubid={}&eid={}",
                                    series_id, issue_id
                                );
                                Some(ComicId::IssueWithMetadata(data_url, metadata))
                            })
                            .rev()
                            .collect::<Option<Vec<ComicId>>>()
                    }
                )
            },
            _ => Err(Error::FailedResponseParse)
        }
    }

    fn metadata_require_authentication(&self) -> bool {
        false
    }

    fn pages_require_authentication(&self) -> bool {
        false
    }

    fn get_pages(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Vec<Page>>> {
        if let ComicId::Issue(url) | ComicId::IssueWithMetadata(url, _) = comicid {
            Ok(SourceResponse::Request(source_request!(
                requests: client.get(url),
                transform: value_fn(&response_to_pages)
            ).unwrap()))
        } else { Err(Error::FailedDownload(self.name())) }
    }

}

fn get_series_data(resp: &[bytes::Bytes], series_id: &str) -> Option<serde_json::Value> {
    let response_data: serde_json::Value = resp_to_json(&resp[0])?;
    // Finding correct series
    let series_data = response_data["publications"]
        .as_array()?
        .to_owned()
        .into_iter()
        .find(|x| x["customPublicationCode"].as_str() == Some(series_id))?;
    Some(series_data)
}

fn signin_data(client: &Client) -> reqwest::RequestBuilder {
    // Required data
    let data = HashMap::from([
        ("email", ""),
        ("password", ""),
        ("token", ""),
        ("languageCulture", "da-DK"),
        ("appId", ""),
        ("appVersion", ""),
        ("uuid", ""),
        ("os", "")
    ]);
    client.post("https://flippapi.egmontservice.com/api/signin")
        .json(&data)
}


fn response_to_pages(responses: &[bytes::Bytes]) -> Option<Vec<Page>> {
    utils::resp_to_json::<serde_json::Value>(&responses[0])?["pageGroups"]
        .as_array()?
        .iter()
        .map(|x| {
            // Finding id from low quality image url
            let low_quality_url = x["pages"][0]["image"].as_str()?;
            let page_id = Regex::new(r"/\w/\w/[^/]+").ok()?
                .find(low_quality_url)?.as_str();
            // Add link to high quality image
            let url = format!("http://pages.cdn.pagesuite.com{}/highpage.jpg?method=true", page_id);
            Some(Page::from_url(&url, "jpg"))
        })
        .collect::<Option<Vec<Page>>>()
}

#[cfg(test)]
mod tests {
    use crate::source::{ComicId, Source};

    #[test]
    fn issue_id() {
        let source = super::Flipp;
        assert_eq!(
            source.id_from_url("https://reader.flipp.dk/html5/reader/production/default.aspx?pubname=&edid=31d29e20-fd60-48ad-96b2-79a3d9d65788").unwrap(),
            ComicId::Other("31d29e20-fd60-48ad-96b2-79a3d9d65788".to_string())
        );
        assert_eq!(
            source.id_from_url("https://magasiner.flipp.dk/flipp/web-app/#/publications/fa7c63ad-0a48-445b-9a17-7d536006902a").unwrap(),
            ComicId::Series("fa7c63ad-0a48-445b-9a17-7d536006902a".to_string())
        );
    }

    #[test]
    fn pages() {
        let responses = std::fs::read("./tests/source_data/flipp_issue.json").unwrap();
        let pages = super::response_to_pages(&[responses.into()]).unwrap();
        assert_eq!(pages.len(), 259);
    }
}
