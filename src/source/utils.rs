use super::{Result, Error, ComicId};

/// User Agent of Chrome on Android
pub const ANDROID_USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 9; ASUS_X00TD; Flow) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/359.0.0.288 Mobile Safari/537.36";

/// Create a `ComicId` from an url and regular expressions. First argument is the url which should
/// be converted. The rest is pairs of regular expressions and `ComicId` types. The first capture
/// group in the regular expression will be used as the id itself. The first matching pair will be
/// used and the rest ignored.
///
/// Example:
/// ```ignore
/// issue_id_match!(url
///     r"viewer\?.+episode_no=(\d+)" => Issue,
///     r"list\?title_no=(\d+)" => Series
/// )
/// ```
macro_rules! issue_id_match {
    ($url:expr, $($pattern:expr => $idtype:ident),+) => {
        crate::source::utils::issue_id_match_internal($url, &[$(
            ($pattern, Box::new(ComicId::$idtype)),
        )*])
    }
}
pub(super) use issue_id_match;

/// Internal function for `issue_id_match` macro. Does most of the work
pub fn issue_id_match_internal(url: &str, pairs: &[(&str, Box<dyn Fn(String) -> ComicId>)]) -> Result<ComicId> {
    for (pattern, id_type) in pairs {
        let re = regex::Regex::new(pattern).unwrap();
        if re.is_match(url) {
            return Ok(id_type(
                first_capture(re, url).ok_or(Error::UrlNotSupported(url.to_string()))?
            ));
        }
    }
    Err(Error::UrlNotSupported(url.to_string()))
}

/// Shorthand for writing return values for many `Source` methods.
/// ```ignore
/// source_request!(
///     requests: client.get(url),
///     transform: <function>
/// )
/// ```
/// will be transformed to
/// ```ignore
/// Ok(Request {
///     requests: vec![client.get(url).build()?],
///     transform: Box::new(<function>)
/// })
/// ```
macro_rules! source_request {
    // Multiple requests
    (requests: [$($request:expr),+], transform: $transform:expr) => {
        Ok::<_, crate::error::GrawlixDownloadError>(crate::source::Request {
            requests: vec![$($request.build()?,)*],
            transform: Box::new($transform)
        })
    };
    // One request
    (requests: $request:expr, transform: $transform:expr) => {
        source_request!(
            requests: [$request],
            transform: $transform
        )
    };
}
pub(super) use source_request;

/// Extract text of the first html element matching the css selector.
pub fn first_text(doc: &scraper::html::Html, selector: &str) -> Option<String> {
    let text = doc.select(&scraper::selector::Selector::parse(selector).unwrap())
        .next()?
        .text().collect();
    return Some(text);
}

/// Converts binary response to json
pub fn resp_to_json<'a, T: serde::Deserialize<'a>>(response: &'a [u8]) -> Option<T> {
    serde_json::from_str(std::str::from_utf8(response).ok()?).ok()
}

/// Converts `serde_json::Value` to `Option<String>`
pub fn value_to_optstring(value: &serde_json::Value) -> Option<String> {
    value.as_str().map(|x| x.to_string())
}

/// Find first matching capture in regex
pub fn first_capture(re: regex::Regex, text: &str) -> Option<String> {
    Some(re.captures(text)?.get(1)?.as_str().to_string())
}
