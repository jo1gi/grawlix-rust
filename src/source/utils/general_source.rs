use crate::{
    metadata::Metadata,
    comic::Page,
    source::{
        Source, ComicId, Result, SourceResponse, SeriesInfo,
        utils::{issue_id_match, source_request}
    },
};
use reqwest::Client;

/// Generic implementation for `crate::source::Source`
pub struct StandardSource {
    /// Name of source
    pub name: String,
    /// Regex that matches issue id
    pub issue_id_regex: String,
    /// Regex that matches series id
    pub series_id_regex: String,
    /// Method for retrieving ids in series
    pub series_id_retrieval_method: RetrievalMethod<Vec<ComicId>>,
}

#[derive(Clone)]
pub enum RetrievalMethod<T> {
    /// Add comicid to url and call transform
    Simple {
        url: String,
        transform: Box<dyn Fn(&[bytes::Bytes]) -> Option<T> + Send>
    }
}

/// Create `SourceResponse` from `RetrievalMethod`
fn apply_retrieval_method<T>(retrieval_method: &RetrievalMethod<T>, client: &Client, id: &str) -> Result<SourceResponse<T>> {
    match retrieval_method {
        RetrievalMethod::Simple{ url, transform } => {
            source_request!(
                requests: client.get(
                    url.replace("{}", id)
                ),
                transform: transform
            )
        }
    }
}

impl Source for StandardSource {

    fn name(&self) -> String {
        self.name.clone()
    }

    fn id_from_url(&self, url: &str) -> Result<ComicId> {
        issue_id_match!(url,
            &self.series_id_regex => Series,
            &self.issue_id_regex => Issue
        )
    }

    fn get_series_ids(&self, client: &Client, seriesid: &ComicId) -> Result<SourceResponse<Vec<ComicId>>> {
        if let ComicId::Series(id) = seriesid {
            apply_retrieval_method(&self.series_id_retrieval_method, client, &id)
        } else { unreachable!() }
    }

    fn get_series_info(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<SeriesInfo>> {
        todo!()
    }

    fn get_metadata(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Metadata>> {
        todo!()
    }

    fn get_pages(&self, client: &Client, comicid: &ComicId) -> Result<SourceResponse<Vec<Page>>> {
        todo!()
    }

}
