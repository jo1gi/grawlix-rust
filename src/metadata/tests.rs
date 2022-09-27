use super::{Metadata, Author, AuthorType};

fn test_metadata() -> Metadata {
    Metadata {
        title: Some(String::from("Moon Knight #1")),
        series: Some(String::from("Moon Knight (2016 - 2018)")),
        publisher: Some(String::from("Marvel")),
        issue_number: Some(1),
        year: Some(2016),
        month: Some(4),
        day: Some(13),
        authors: vec![
            Author { name: "Jeff Lemire".to_string(), author_type: AuthorType::Writer },
            Author { name: "Greg Smallwood".to_string(), author_type: AuthorType::CoverArtist },
            Author { name: "Greg Smallwood".to_string(), author_type: AuthorType::Penciller },
        ],
        ..Default::default()
    }
}

#[test]
/// Tests if metadata can be correctly exported in comicinfo.xml format
fn comicrack_export() {
    assert_eq!(
        test_metadata().comicrack().unwrap(),
        std::fs::read_to_string("./tests/metadata_data/comicrack.xml").unwrap().trim()
    );
}

#[test]
/// Tests if metadata can be correctly imported from comicrack format
fn comicrack_import() {
    let input = std::fs::read_to_string("./tests/metadata_data/comicrack.xml").unwrap();
    assert_eq!(Metadata::from_comicrack_str(input.as_ref()), test_metadata());
}

#[test]
fn date_from_str() {
    assert_eq!(
        super::date_from_str("2022-09-27"),
        Some((2022,09,27))
    );
}
