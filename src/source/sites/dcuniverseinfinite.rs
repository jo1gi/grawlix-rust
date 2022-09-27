use crate::{
    comic::{Page, PageType, PageEncryptionScheme, OnlinePage},
    metadata::{Metadata, Author, AuthorType},
    source::{
        Source, Result, Error, ComicId, SeriesInfo, Request, SourceResponse, Credentials,
        utils::{issue_id_match, simple_response, simple_request, source_request, resp_to_json, value_fn}
    }
};
use reqwest::{Client, header};
use crypto::{
    sha2::Sha256,
    digest::Digest
};
use log::debug;

#[derive(Default)]
pub struct DCUniverseInfinite {
    authorization_key: Option<String>
}

#[async_trait::async_trait]
impl Source for DCUniverseInfinite {

    fn name(&self) -> String {
        "DC Universe Infinite".to_string()
    }

    fn create_client(&self) -> Client {
        let mut headers = header::HeaderMap::new();
        headers.insert("X-Consumer-Key", header::HeaderValue::from_static("DA59dtVXYLxajktV"));
        if let Some(x) = &self.authorization_key {
            headers.insert("Authorization", header::HeaderValue::from_str(&format!("Token {}", x)).unwrap());
        }
        Client::builder()
            .default_headers(headers)
            .build()
            .unwrap()
    }

    fn id_from_url(&self, url: &str) -> Result<ComicId> {
        issue_id_match!(url,
            r"comics/book/[^/]+/([^/]+)" => Issue,
            r"comics/series/[^/]+/([^/]+)" => Series
        )
    }

    fn get_series_ids(&self, client: &Client, seriesid: &ComicId) -> Result<Request<Vec<ComicId>>> {
        simple_request!(
            id: seriesid,
            client: client,
            id_type: Series,
            url: "https://www.dcuniverseinfinite.com/api/comics/1/series/{}/?trans=en",
            transform: find_series_ids
        )
    }

    fn get_series_info(&self, client: &Client, seriesid: &ComicId) -> Result<SourceResponse<SeriesInfo>> {
        simple_response!(
            id: seriesid,
            client: client,
            id_type: Series,
            url: "https://www.dcuniverseinfinite.com/api/comics/1/series/{}/?trans=en",
            value: parse_series_info
        )
    }

    fn get_metadata(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Metadata>> {
        simple_response!(
            id: comicid,
            client: client,
            id_type: Issue,
            url: "https://www.dcuniverseinfinite.com/api/comics/1/book/{}/?trans=en",
            value: parse_metadata
        )
    }

    fn get_pages(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Vec<Page>>> {
        let new_client = client.clone();
        simple_response!(
            id: comicid,
            client: client,
            id_type: Issue,
            url: "https://www.dcuniverseinfinite.com/api/5/1/rights/comic/{}?trans=en",
            request: move |resp| {
                let auth_jwt = resp_to_json::<serde_json::Value>(&resp[0])?;
                debug!("auth_jwt: {}", auth_jwt.as_str()?);
                Some(crate::source::SourceResponse::Request(crate::source::Request {
                    requests: vec![
                        new_client
                            .get("https://www.dcuniverseinfinite.com/api/comics/1/book/download/?page=1&quality=HD&trans=en")
                            .header("X-Auth-JWT", auth_jwt.as_str()?)
                    ],
                    transform: value_fn(&create_pages)
                }))
            }
        )
    }

    async fn authenticate(&mut self, _client: &mut Client, creds: &Credentials) -> Result<()> {
        if let Credentials::ApiKey(apikey) = creds {
            self.authorization_key = Some(apikey.clone());
            Ok(())
        } else {
            Err(Error::FailedAuthentication("DC Universe Unlimited requires an api key to login".to_string()))
        }
    }
}

fn find_series_ids(resp: &[bytes::Bytes]) -> Option<Vec<ComicId>> {
    let data = resp_to_json::<serde_json::Value>(&resp[0])?;
    data["book_uuids"]["issue"]
        .as_array()?
        .into_iter()
        .map(|x| Some(ComicId::Issue(x.as_str()?.to_string())))
        .collect()
}

fn create_pages(resp: &[bytes::Bytes]) -> Option<Vec<Page>> {
    let data = resp_to_json::<serde_json::Value>(&resp[0])?;
    let uuid = data["uuid"].as_str()?;
    let job_id = data["job_id"].as_str()?;
    let format = data["format"].as_str()?;
    data["images"]
        .as_array()?
        .into_iter()
        .map(|x| {
            Some(Page {
                file_format: "jpg".to_string(),
                page_type: PageType::Url(OnlinePage {
                    url: x["signed_url"].as_str()?.to_string(),
                    headers: None,
                    encryption: Some(PageEncryptionScheme::DCUniverseInfinite(
                        create_decryption_key(uuid, x["page_number"].as_u64()?, job_id, format)
                    ))
                })
            })
        })
        .collect()
}

fn parse_metadata(resp: &[bytes::Bytes]) -> Option<Metadata> {
    let data = resp_to_json::<serde_json::Value>(&resp[0])?;
    let author_fn = |field: &str, author_type: AuthorType| -> Option<Vec<Author>> {
        Some(data[field]
            .as_array()?
            .into_iter()
            .filter_map(|x| Some(Author {
                name: x["display_name"].as_str()?.to_string(),
                author_type: author_type.clone()
            }))
            .collect())
    };
    let mut authors = Vec::new();
    authors.append(&mut author_fn("authors", AuthorType::Writer)?);
    authors.append(&mut author_fn("colorists", AuthorType::Colorist)?);
    authors.append(&mut author_fn("cover_artists", AuthorType::CoverArtist)?);
    authors.append(&mut author_fn("inkers", AuthorType::Inker)?);
    authors.append(&mut author_fn("pencillers", AuthorType::Penciller)?);
    Some(Metadata {
        title: data["title"].as_str().map(String::from),
        series: data["series_title"].as_str().map(String::from),
        description: data["description"].as_str().map(String::from),
        publisher: data["publisher"].as_str().map(String::from),
        issue_number: data["issue_number"].as_str().and_then(|x| x.parse::<u32>().ok()),
        authors,
        ..Default::default()
    })
}

fn parse_series_info(resp: &[bytes::Bytes]) -> Option<SeriesInfo> {
    let data = resp_to_json::<serde_json::Value>(&resp[0])?;
    Some(SeriesInfo {
        name: data["title"].as_str()?.to_string()
    })
}

/// Create decryption key for pages
fn create_decryption_key(uuid: &str, page_number: u64, job_id: &str, format_id: &str) -> [u8; 32] {
    let string_key = format!("{}{}{}{}", uuid, page_number, job_id, format_id);
    let mut hasher = Sha256::new();
    hasher.input_str(&string_key);
    let mut key: [u8; 32] = [0; 32];
    hasher.result(&mut key);
    return key;
}

#[cfg(test)]
mod tests {
    use crate::source::{Source, ComicId};

    #[test]
    fn ids() {
        let source = super::DCUniverseInfinite::default();
        assert_eq!(
            source.id_from_url(
                "https://www.dcuniverseinfinite.com/comics/book/the-sandman-8/761ad52d-b961-49b1-87b6-ca85774fc3a6/c/reader"
            ).unwrap(),
            ComicId::Issue("761ad52d-b961-49b1-87b6-ca85774fc3a6".to_string())
        );
        assert_eq!(
            source.id_from_url(
                "https://www.dcuniverseinfinite.com/comics/series/the-sandman/fbf5f10f-03ca-4f2b-90a0-66df08806a99"
            ).unwrap(),
            ComicId::Series("fbf5f10f-03ca-4f2b-90a0-66df08806a99".to_string())
        );
    }

    #[test]
    fn decryption_key() {
        let key = super::create_decryption_key(
            "761ad52d-b961-49b1-87b6-ca85774fc3a6",
            1,
            "fcc51f44-4a82-47b1-9eac-13f3f1068571",
            "HD"
        );
        assert_eq!(
            key,
            [221,142,219,226,164,30,108,77,254,230,3,14,0,205,167,253,92,26,25,15,124,214,129,246,39,14,89,51,223,155,54,225]
        )
    }

    #[test]
    fn metadata() {
        let resp = std::fs::read("./tests/source_data/dcuniverseinfinite_issue.json").unwrap();
        let metadata = super::parse_metadata(&[resp.into()]).unwrap();
        assert_eq!(
            metadata,
            crate::metadata::Metadata {
                title: Some("The Sandman #8".to_string()),
                series: Some("The Sandman".to_string()),
                description: Some("Spend a day with Dream as he catches up with his younger sister, Death, in search of inspiration. When the King of Dreams is depressed, can even a pep talk from Death set him on the right path?".to_string()),
                publisher: Some("DC Comics".to_string()),
                issue_number: Some(8),
                authors: vec![
                    super::Author { name: "Neil Gaiman".to_string(), author_type: super::AuthorType::Writer },
                    super::Author { name: "Robbie Busch".to_string(), author_type: super::AuthorType::Colorist },
                    super::Author { name: "Dave McKean".to_string(), author_type: super::AuthorType::CoverArtist },
                    super::Author { name: "Malcolm Jones III".to_string(), author_type: super::AuthorType::Inker },
                    super::Author { name: "Mike Dringenberg".to_string(), author_type: super::AuthorType::Penciller },
                ],
                ..Default::default()
            }
        );
    }

    #[test]
    fn series_ids() {
        let resp = std::fs::read("./tests/source_data/dcuniverseinfinite_series.json").unwrap();
        let issues = super::find_series_ids(&[resp.into()]).unwrap();
        assert_eq!(issues.len(), 8);
        assert_eq!(issues[2], ComicId::Issue("1958170b-f678-4eeb-a774-ef750b8aa8bc".to_string()));
    }

    #[test]
    fn series_info() {
        let resp = std::fs::read("./tests/source_data/dcuniverseinfinite_series.json").unwrap();
        let info = super::parse_series_info(&[resp.into()]).unwrap();
        assert_eq!(&info.name, "The Sandman");
    }
}
