use super::{ComicId, Source, Request, SourceResponse, Result, Error, SeriesInfo};
use crate::{
    comic::Comic, metadata::{Metadata, Identifier}
};
use async_recursion::async_recursion;
use futures::{StreamExt, TryStreamExt, stream};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue}
};
use log::{debug, trace};

/// Create new default `reqwest::Client` to use in `Source`
pub fn create_default_client() -> reqwest::Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        "User-Agent",
        HeaderValue::from_static("grawlix")
    );
    Client::builder()
        .default_headers(headers)
        .build()
        .unwrap()
}

/// Download all comics from url
pub async fn download_comics_from_url(url: &str) -> Result<Vec<Comic>> {
    let source = super::source_from_url(url)?;
    let mut client = source.create_client();
    let comicid = source.id_from_url(url)?;
    debug!("Got id from url: {:?}", comicid);
    let all_ids = get_all_ids(&source, &mut client, comicid).await?;
    download_comics(all_ids, &client, &source).await
}

/// Downloads `Metadata` from comicid if `Issue` and extracts metadata if `IssueWithMetadata` and
/// adds identifier for current source
async fn metadata_from_comicid(source: &Box<dyn Source>, client: &Client, comicid: ComicId) -> Result<Metadata> {
    let id_str = comicid.inner().clone(); // Needed later
    // Extract or download metadata
    let mut metadata = match comicid {
        ComicId::Issue(_) => {
            let metadata_response = source.get_metadata(&client, &comicid)?;
            eval_source_response(metadata_response).await?
        },
        ComicId::IssueWithMetadata(_, meta) => meta,
        _ => unreachable!()
    };
    // Add identifier for current source
    metadata.identifiers.push(Identifier {
        source: source.name(),
        id: id_str
    });
    Ok(metadata)
}

/// Creates `Comic` from comicid
pub async fn comic_from_comicid(source: &Box<dyn Source>, client: &Client, comicid: ComicId) -> Result<Comic> {
    let pages_response = source.get_pages(&client, &comicid)?;
    log::trace!("Retrieving pages");
    let pages = eval_source_response(pages_response).await?;
    log::trace!("Retrieving metadata");
    let metadata = metadata_from_comicid(source, client, comicid).await?;
    Ok(Comic {
        pages,
        metadata,
        ..Default::default()
    })
}

/// Download all comics from ids
pub async fn download_comics(comic_ids: Vec<ComicId>, client: &Client, source: &Box<dyn Source>) -> Result<Vec<Comic>> {
    stream::iter(comic_ids)
        .map(|comicid| {
            let source = &source;
            let client = &client;
            async move {
                comic_from_comicid(source, client, comicid).await
            }
        })
        .buffered(5)
        .try_collect()
        .await
}

/// Download series metadata
pub async fn download_series_metadata(client: &Client, source: &Box<dyn Source>, comicid: &ComicId) -> Result<SeriesInfo> {
    let request = source.get_series_info(client, comicid)?;
    let series_info = eval_source_response(request).await?;
    Ok(series_info)
}

pub async fn download_comics_metadata(
    source: &mut Box<dyn Source>,
    url: &str,
) -> Result<Vec<Metadata>> {
    let mut client = source.create_client();
    let comicid = source.id_from_url(url)?;
    let all_ids = get_all_ids(&source, &mut client, comicid).await?;
    let mut metadata = Vec::new();
    for i in all_ids {
        let response = source.get_metadata(&client, &i)?;
        let content = eval_source_response(response).await?;
        metadata.push(content);
    }
    return Ok(metadata);
}

async fn eval_source_response<T>(response: SourceResponse<T>) -> Result<T> {
    let mut response = response;
    loop {
        match response {
            SourceResponse::Value(v) => return Ok(v),
            SourceResponse::Request(r) => {
                response = make_request(r).await?;
            }
        }
    }
}

async fn make_request<T>(request: Request<T>) -> Result<T> {
    let mut responses = Vec::new();
    trace!("Making request");
    for request in request.requests {
        let bytes = request
            .send()
            .await?
            .bytes()
            .await?;
        responses.push(bytes);
    }
    trace!("Transforming response");
    (request.transform)(&responses).ok_or(Error::FailedResponseParse)
}

#[async_recursion(?Send)]
pub async fn get_all_ids(
    source: &Box<dyn Source>,
    client: &Client,
    comicid: ComicId
) -> Result<Vec<ComicId>> {
    Ok(match comicid {
        ComicId::Other(_) => {
            let new_id_request = source.get_correct_id(client, &comicid)?;
            let new_id = eval_source_response(new_id_request).await?;
            get_all_ids(source, client, new_id).await?
        },
        ComicId::OtherWithMetadata(id, meta) => {
            let new_ids = get_all_ids(source, client, ComicId::Other(id)).await?;
            match &new_ids[..] {
                [ComicId::Issue(x)] => vec![ComicId::IssueWithMetadata(x.to_string(), meta)],
                _ => new_ids,
            }
        }
        ComicId::Series(_) => {
            // Ids of each issue in series
            let new_ids = eval_source_response(source.get_series_ids(client, &comicid)?).await?;
            // let mut result = Vec::new();
            let evaluated_ids = stream::iter(new_ids)
                .map(|new_id| async move {
                    get_all_ids(source, client, new_id).await
                })
                .buffered(5)
                .collect::<Vec<Result<Vec<ComicId>>>>().await;
            // Evaluating new ids
            let mut result = Vec::new();
            for id in evaluated_ids {
                result.append(&mut id?);
            }
            debug!("Finished downloading series ids for {:?}", comicid);
            result
        },
        ComicId::Issue(_) => vec![comicid],
        ComicId::IssueWithMetadata(..) => vec![comicid],
    })
}
