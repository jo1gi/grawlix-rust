use super::{Metadata, Author, AuthorType};

pub fn test_metadata() -> Metadata {
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
fn date_from_str() {
    assert_eq!(
        super::date_from_str("2022-09-27"),
        Some((2022,09,27))
    );
}
