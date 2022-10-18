use super::{Metadata, Author};
use xml::{
    reader::{ParserConfig, XmlEvent as ReaderEvent},
    writer::{XmlEvent as WriterEvent, EmitterConfig, EventWriter, Error as WriteError}
};

/// Write a tag and string to xml writer
fn write_simple<W: std::io::Write>(
    writer: &mut EventWriter<W>,
    tag: &str,
    content: &str
) -> Result<(), WriteError> {
    writer.write(WriterEvent::start_element(tag))?;
    writer.write(content)?;
    writer.write(WriterEvent::end_element())?;
    Ok(())
}

/// Write an tag and content to xml writer if content is some
fn write_option<W: std::io::Write, S: ToString>(
    writer: &mut EventWriter<W>,
    tag: &str, content: &Option<S>
) -> Result<(), WriteError> {
    if let Some(c) = content {
        write_simple(writer, tag, &c.to_string())?;
    }
    Ok(())
}

/// Export metadata in comicrack (comicinfo.xml) format
pub fn export(metadata: &Metadata) -> Result<String, WriteError> {
    let mut buffer = Vec::new();
    {
        let mut w = EmitterConfig::new()
            .perform_indent(true)
            .create_writer(&mut buffer);
        w.write(WriterEvent::start_element("ComicInfo"))?;
        write_option(&mut w, "Title", &metadata.title)?;
        write_option(&mut w, "Series", &metadata.series)?;
        write_option(&mut w, "Publisher", &metadata.publisher)?;
        write_option(&mut w, "Number", &metadata.issue_number)?;
        write_option(&mut w, "Year", &metadata.year)?;
        write_option(&mut w, "Month", &metadata.month)?;
        write_option(&mut w, "Day", &metadata.day)?;
        for author in &metadata.authors {
            write_simple(&mut w, author.author_type.to_string().as_ref(), author.name.as_ref())?
        }
        w.write(WriterEvent::end_element())?;
    }
    let output = std::str::from_utf8(buffer.as_slice()).unwrap().to_string();
    return Ok(output);
}

/// Create new Metadata object from comicinfo.xml
pub fn import<R: std::io::Read>(source: R) -> Metadata {
    let parser = ParserConfig::new()
        .ignore_comments(true)
        .whitespace_to_characters(true)
        .cdata_to_characters(false)
        .trim_whitespace(true)
        .create_reader(source);
    let mut new: Metadata = Default::default();
    let mut current = String::new();
    for e in parser {
        match e {
            Ok(ReaderEvent::StartElement { name, .. }) => {
                current = name.local_name;
            },
            Ok(ReaderEvent::Characters(content)) => {
                match current.as_str() {
                    "Title" => new.title = Some(content),
                    "Series" => new.series = Some(content),
                    "Publisher" => new.publisher = Some(content),
                    "Number" => new.issue_number = content.parse().ok(),
                    "Year" => new.year = content.parse().ok(),
                    "Month" => new.month = content.parse().ok(),
                    "Day" => new.day = content.parse().ok(),
                    "Writer" | "Penciller" | "Inker" | "Colorist" | "Letterer" | "CoverArtist" | "Editor" =>
                        new.authors.push(Author{name:content, author_type: current.clone().into()}),
                    _ => (),
                }
            }
            _ => (),
        }
    }
    return new;
}

pub fn import_str(source: &str) -> Metadata {
    import(source.as_bytes())
}

#[cfg(test)]
mod test {
    use crate::metadata::tests::test_metadata;

    /// Tests if metadata can be correctly exported in comicinfo.xml format
    #[test]
    fn comicrack_export() {
        assert_eq!(
            super::export(&test_metadata()).unwrap(),
            std::fs::read_to_string("./tests/metadata_data/comicrack.xml").unwrap().trim()
        );
    }

    /// Tests if metadata can be correctly imported from comicrack format
    #[test]
    fn comicrack_import() {
        let input = std::fs::read_to_string("./tests/metadata_data/comicrack.xml").unwrap();
        assert_eq!(super::import_str(input.as_ref()), test_metadata());
    }

}
