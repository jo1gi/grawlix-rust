use crate::source::SourceResponse;

/// Read source testdata file and convert to array of bytes with one entry
pub fn response_from_testfile(testfile: &str) -> [bytes::Bytes; 1] {
    let data = std::fs::read(format!("./tests/source_data/{}", testfile)).unwrap();
    [data.into()]
}

/// Extract transform function from request in SourceResponse
pub fn transform_from_source_response<T: 'static>(
    source_response: Result<SourceResponse<T>, crate::error::GrawlixDownloadError>
) -> Box<dyn Fn(&[bytes::Bytes]) -> T> {
    match source_response.unwrap() {
        SourceResponse::Request(request) => Box::new(move |resp| {
            match ((request.transform)(resp)).unwrap() {
                SourceResponse::Value(v) => v,
                SourceResponse::Request(_) => panic!("Response requires another request")
            }
        }),
        SourceResponse::Value(_) => panic!("Response is not a request")
    }
}
