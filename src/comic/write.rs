use crate::error::GrawlixIOError as Error;
use super::{Comic, ComicFormat, PageType};
use std::{
    io::prelude::Write,
    path::Path,
};


impl Comic {

    /// Write comic book to disk
    pub async fn write(&self, template: &str, comic_format: ComicFormat) -> Result<(), Error> {
        let client = reqwest::Client::new();
        let path = self.format(template)?;
        let mut comic_file = new_comic_file(&path, &comic_format)?;
        for (n, page) in self.pages.iter().enumerate() {
            // Getting page data
            let page_data = match &page.page_type {
                // Download page
                PageType::Url(url) => client.get(url).send().await.unwrap().bytes().await.unwrap(),
                // Skipping rewriting pages already stored in file
                _ => continue,
            };
            let filename = format!("{} #{:0>3}.{}", self.title(), n, &page.file_format);
            comic_file.write_file(&page_data, &filename)?;
        }
        for (name, data) in self.metadata.export_all()? {
            comic_file.write_file(&data.as_bytes(), name)?;
        }
        comic_file.finish()?;
        Ok(())
    }

}

/// Create new output container for comic
fn new_comic_file(path_str: &str, format: &ComicFormat) -> Result<Box<dyn ComicFile>, Error> {
    // Finding path
    let path = Path::new(path_str);
    // Creating parent dir if it does not exist
    let parent = path.parent().ok_or(Error::InvalidLocation(path_str.to_string()))?;
    if !parent.exists() {
        std::fs::create_dir_all(parent).or(Err(Error::InvalidLocation(path_str.to_string())))?;
    }
    Ok(Box::new(match format {
        ComicFormat::CBZ => {
            let file = std::fs::File::create(&path)?;
            let zip = zip::ZipWriter::new(file);
            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            ZipComic {zip, options}
        }
    }))
}

/// Specifies an output container a comic can be written to
trait ComicFile {
    /// Write file to container
    fn write_file(&mut self, data: &[u8], name: &str) -> Result<(), Error>;
    /// Finish writing to container
    fn finish(&mut self) -> Result<(), Error>;
}

/// Zip formatted comic book output
struct ZipComic {
    zip: zip::ZipWriter<std::fs::File>,
    options: zip::write::FileOptions,
}

impl ComicFile for ZipComic {
    fn write_file(&mut self, data: &[u8], name: &str) -> Result<(), Error> {
        self.zip.start_file(name, self.options)?;
        self.zip.write_all(data)?;
        Ok(())
    }
    fn finish(&mut self) -> Result<(), Error> {
        self.zip.finish()?;
        Ok(())
    }
}
