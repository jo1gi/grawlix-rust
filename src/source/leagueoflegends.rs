use crate::{
    source::{
        Source, ComicId, Result, SourceResponse, Request, Error, SeriesInfo,
        utils::{issue_id_match, source_request, resp_to_json}
    },
    comic::Page,
    metadata::{Metadata, Author, AuthorType},
};
use reqwest::Client;


pub struct LeagueOfLegends;

impl Source for LeagueOfLegends {
    fn name(&self) -> String {
        "League of Legends".to_string()
    }

    fn id_from_url(&self, url: &str) -> Result<ComicId> {
        issue_id_match!(url,
            r"/comic/([^/]+/[^/]+)/" => Issue,
            r"/comic/([^/]+)" => Series
        )
    }

    fn get_series_ids(&self, client: &Client, seriesid: &ComicId) -> Result<Request<Vec<ComicId>>> {
        if let ComicId::Series(id) = seriesid {
            let sid = id.clone();
            source_request!(
                requests: client.get(info_url(id)),
                transform: move |responses| {
                    resp_to_json::<serde_json::Value>(&responses[0])?
                        .get("issues")?
                        .as_array()?
                        .iter()
                        .map(|issue| {
                            Some(ComicId::Issue(format!("{}/{}", sid, issue["id"].as_str()?)))
                        })
                        .collect()
                }
            )
        } else { Err(Error::FailedResponseParse) }
    }

    fn get_series_info(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<SeriesInfo>> {
        if let ComicId::Series(x) = comicid {
            Ok(SourceResponse::Request(source_request!(
                requests: client.get(info_url(x)),
                transform: response_series_info
            ).unwrap()))
        } else { Err(Error::FailedResponseParse) }
    }

    fn metadata_require_authentication(&self) -> bool {
        false
    }

    fn pages_require_authentication(&self) -> bool {
        false
    }

    fn get_metadata(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Metadata>> {
        if let ComicId::Issue(issueid) = comicid {
            Ok(SourceResponse::Request(source_request!(
                requests: client.get(info_url(issueid)),
                transform: response_to_metadata
            ).unwrap()))
        } else { Err(Error::FailedResponseParse) }
        // Ok(SourceResponse::Value(Metadata::default()))
    }


    fn get_pages(&self, client: &Client, comicid: &ComicId) -> Result<Request<Vec<Page>>> {
        if let ComicId::Issue(issueid) = comicid {
            source_request!(
                requests: [
                    // Pages
                    client.get(
                        format!(
                            "https://universe-comics.leagueoflegends.com/comics/en_us/{}/index.json",
                            issueid
                        )
                    ),
                    // Metadata includes cover page
                    client.get(info_url(issueid))
                ],
                transform: response_to_pages
            )
        } else { Err(Error::FailedResponseParse) }
    }

}

fn info_url(id: &str) -> String {
    format!(
        "https://universe-meeps.leagueoflegends.com/v1/en_us/comics/{}/index.json",
        id
    )
}

fn response_series_info(responses: &[bytes::Bytes]) -> Option<SeriesInfo> {
    Some(SeriesInfo {
        name: resp_to_json::<serde_json::Value>(&responses[0])?
            .get("name")?
            .as_str()?
            .to_string()
    })
}

fn response_to_pages(responses: &[bytes::Bytes]) -> Option<Vec<Page>> {
    let mut pages = resp_to_json::<serde_json::Value>(&responses[0])?
        .get("desktop-pages")?
        .as_array()?
        .iter()
        // Combining lists
        // TODO Improve
        .map(|x| Some(x.as_array()?.clone()))
        .collect::<Option<Vec<Vec<serde_json::Value>>>>()?
        .iter()
        .flatten()
        // Extracting pages
        .map(|issue| {
            Some(Page::from_url(issue["uri"].as_str()?, "jpg"))
        })
        .collect::<Option<Vec<Page>>>()?;
    let info = resp_to_json::<serde_json::Value>(&responses[1])?;
    let cover_url = info["comic-info"]["cover-image"]["uri"].as_str()?;
    let cover_page = Page::from_url(cover_url, "jpg");
    pages.insert(0, cover_page);
    Some(pages)
}

fn response_to_metadata(responses: &[bytes::Bytes]) -> Option<Metadata> {
    let resp = resp_to_json::<serde_json::Value>(&responses[0])?;
    let info = resp.get("comic-info")?;
    let title = info.get("title")?.as_str()?;
    Some(Metadata {
        title: info["issue-title"].as_str().map(String::from),
        series: info["issue-title"]
            .as_str()
            .map(|x| x.replace(&format!(": {}", title), ""))
        ,
        issue_number: info.get("index")
            .map(|x| Some(x.as_u64()? as u32))
            .flatten(),
        authors: info.get("credits")?
            .as_array()?
            .iter()
            .filter_map(|credit| {
                let author_type = AuthorType::from(credit["credit-label"].as_str()?);
                Some(Author {
                    name: credit["credit-info"].as_str()?.to_string(),
                    author_type,
                })
            })
            .filter(|author| author.author_type != AuthorType::Other)
            .collect(),
        source: Some("League of Legends".to_string()),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use crate::source::Source;
    use crate::metadata::{Author, AuthorType};

    #[test]
    fn metadata() {
        let resp = std::fs::read("./tests/source_data/leagueoflegends_issue_metadata.json").unwrap();
        let metadata = super::response_to_metadata(&[resp.into()]).unwrap();
        assert_eq!(
            metadata,
            crate::metadata::Metadata {
                title: Some("Steadfast Heart: Issue #1".to_string()),
                series: Some("Steadfast Heart".to_string()),
                issue_number: Some(1),
                authors: vec![
                    Author { name: "Ian St. Martin".to_string(), author_type: AuthorType::Writer },
                    Author { name: "Linky of 2:10 Animation".to_string(), author_type: AuthorType::Inker },
                    Author { name: "Bruce Jackie of 2:10 Animation".to_string(), author_type: AuthorType::Colorist },
                    Author { name: "Molly Mahan".to_string(), author_type: AuthorType::Editor },
                ],
                source: Some("League of Legends".to_string()),
                ..Default::default()
            }
        )
    }

    #[test]
    fn pages() {
        let meta_resp = std::fs::read("./tests/source_data/leagueoflegends_issue_metadata.json").unwrap();
        let page_resp = std::fs::read("./tests/source_data/leagueoflegends_issue.json").unwrap();
        let pages = super::response_to_pages(&[page_resp.into(), meta_resp.into()]).unwrap();
        assert_eq!(pages.len(), 11);
    }

    #[test]
    fn series() {
        // Setup
        let source = super::LeagueOfLegends;
        let seriesid = source.id_from_url("https://universe.leagueoflegends.com/en_us/comic/sentinelsoflight")
            .unwrap();
        let client = reqwest::Client::new();
        let data = std::fs::read("./tests/source_data/leagueoflegends_series.json").unwrap();
        let responses = [data.into()];
        // Series issues
        let transform = source.get_series_ids(&client, &seriesid).unwrap().transform;
        let issues = transform(&responses).unwrap();
        assert_eq!(issues.len(), 6);
        if let super::ComicId::Issue(issueid) = &issues[3] {
            assert_eq!("sentinelsoflight/issue-4", issueid);
        } else { panic!("Returned id was not an issue") }
        // Series info
        let series_info = super::response_series_info(&responses).unwrap();
        assert_eq!(series_info.name, "Steadfast Heart".to_string());
    }
}
