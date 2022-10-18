use serde::{Deserialize, Serialize};
use crate::{
    metadata::{Metadata, Author, AuthorType},
    error::GrawlixIOError
};

#[derive(Default, Deserialize, Serialize)]
pub struct TachayomiDetails {
    title: Option<String>,
    author: Option<String>,
    artist: Option<String>,
    description: Option<String>,
    genre: Vec<String>,
}

/// Find author with the given type
fn find_author(authors: &Vec<Author>, author_type: &AuthorType) -> Option<String> {
    authors.iter()
        .find(|author| &author.author_type == author_type)
        .map(|author| author.name.clone())
}

/// Export to Tachayomi metadata
/// https://tachiyomi.org/help/guides/local-manga/#advanced
pub fn export(metadata: &Metadata) -> Result<String, GrawlixIOError> {
    let details = TachayomiDetails {
        title: metadata.title.clone(),
        description: metadata.description.clone(),
        author: find_author(&metadata.authors, &AuthorType::Writer),
        genre: metadata.genres.clone(),
        ..Default::default()
    };
    serde_json::to_string(&details)
        .or(Err(GrawlixIOError::MetadataExport("Tachayomi".to_string())))
}

/// Import from Tachayommi format
/// https://tachiyomi.org/help/guides/local-manga/#advanced
pub fn import<R: std::io::Read>(source: R) -> Result<Metadata, GrawlixIOError> {
    serde_json::from_reader(source)
        .or(Err(GrawlixIOError::MetadataImport("Tachayomi".to_string())))
}

#[cfg(test)]
mod test {
    use crate::metadata::tests::test_metadata;

    #[test]
    fn export() {
        assert_eq!(
            &super::export(&test_metadata()).unwrap(),
            r#"{"title":"Moon Knight #1","author":"Jeff Lemire","artist":null,"description":null,"genre":[]}"#
        );
    }
}
