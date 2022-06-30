use super::{ComicId, Source, Request, SourceResponse, Result, Error, SeriesInfo};
use crate::{
    comic::Comic, metadata::Metadata
};
use async_recursion::async_recursion;
use futures::{StreamExt, TryStreamExt, stream};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue}
};
use log::debug;

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
    let all_ids = get_all_ids(&mut client, comicid, &source).await?;
    download_comics(all_ids, &client, &source).await
}

/// Download all comics from ids
pub async fn download_comics(comic_ids: Vec<ComicId>, client: &Client, source: &Box<dyn Source>) -> Result<Vec<Comic>> {
    stream::iter(comic_ids)
        .map(|i| {
            let source = &source;
            let client = &client;
            async move {
                let pages_response = source.get_pages(&client, &i)?;
                let pages = make_request(&client, pages_response).await?;
                let metadata = match i {
                    ComicId::Issue(_) => {
                        let metadata_response = source.get_metadata(&client, &i)?;
                        eval_source_response(&client, metadata_response).await?
                    },
                    ComicId::IssueWithMetadata(_, meta) => meta,
                    _ => unreachable!()
                };
                Ok(Comic {
                    pages,
                    metadata,
                    ..Default::default()
                })
            }
        })
        .buffered(5)
        .try_collect()
        .await
}

/// Download series metadata
pub async fn download_series_metadata(client: &Client, source: &Box<dyn Source>, comicid: &ComicId) -> Result<SeriesInfo> {
    let request = source.get_series_info(client, comicid)?;
    let series_info = eval_source_response(&client, request).await?;
    Ok(series_info)
}

pub async fn download_comics_metadata(
    source: &mut Box<dyn Source>,
    url: &str,
) -> Result<Vec<Metadata>> {
    let mut client = source.create_client();
    let comicid = source.id_from_url(url)?;
    let all_ids = get_all_ids(&mut client, comicid, &source).await?;
    let mut metadata = Vec::new();
    for i in all_ids {
        let response = source.get_metadata(&client, &i)?;
        let content = eval_source_response(&mut client, response).await?;
        metadata.push(content);
    }
    return Ok(metadata);
}

pub async fn authenticate_source(

) -> Result<()> {
    Ok(())
}

async fn eval_source_response<T>(client: &Client, response: SourceResponse<T>) -> Result<T> {
    match response {
        SourceResponse::Value(v) => Ok(v),
        SourceResponse::Request(r) => make_request(client, r).await
    }
}

async fn make_request<T>(client: &Client, request: Request<T>) -> Result<T> {
    let mut responses = Vec::new();
    for i in request.requests {
        let response = client.execute(i).await?;
        let bytes = response.bytes().await?;
        responses.push(bytes);
    }
    (request.transform)(&responses).ok_or(Error::FailedResponseParse)
}

#[async_recursion(?Send)]
pub async fn get_all_ids(
    client: &Client,
    comicid: ComicId,
    source: &Box<dyn Source>
) -> Result<Vec<ComicId>> {
    Ok(match comicid {
        ComicId::Other(_) => {
            let new_id = make_request(client, source.get_correct_id(client, &comicid)?).await?;
            get_all_ids(client, new_id, source).await?
        },
        ComicId::OtherWithMetadata(id, meta) => {
            let new_ids = get_all_ids(client, ComicId::Other(id), source).await?;
            match &new_ids[..] {
                [ComicId::Issue(x)] => vec![ComicId::IssueWithMetadata(x.to_string(), meta)],
                _ => new_ids,
            }
        }
        ComicId::Series(_) => {
            // Ids of each issue in series
            let new_ids = make_request(client, source.get_series_ids(client, &comicid)?).await?;
            // let mut result = Vec::new();
            let evaluated_ids = stream::iter(new_ids)
                .map(|new_id| async move {
                    get_all_ids(client, new_id, source).await
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
