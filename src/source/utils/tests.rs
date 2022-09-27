/// Read source testdata file and convert to array of bytes with one entry
pub fn response_from_testfile(testfile: &str) -> [bytes::Bytes; 1] {
    let data = std::fs::read(format!("./tests/source_data/{}", testfile)).unwrap();
    [data.into()]
}
