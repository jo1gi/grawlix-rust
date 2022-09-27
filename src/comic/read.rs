use crate::{
    error::GrawlixIOError as Error,
    comic::{Comic, Page},
    metadata::Metadata
};

static IMAGE_EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

impl super::Comic {

    /// Create `Comic` object from file
    pub fn from_file(path: &str) -> Result<Self, Error> {
        if path.ends_with(".cbz") || path.ends_with(".zip") {
            Self::from_cbz_file(path)
        } else {
            Err(Error::UnknownFileType(path.to_string()))
        }
    }

    /// Create `Comic` object from cbz file
    fn from_cbz_file(path: &str) -> Result<Self, Error> {
        // Loading zip file
        let file = std::fs::File::open(path)?;
        let mut zip = zip::ZipArchive::new(file)?;
        // Creating `Comic` object
        let mut comic = Comic::default();
        // Adding files
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let name = file.name().to_string();
            let path = std::path::Path::new(file.name());
            // Add file as page
            if let Some(ext) = path.extension() {
                if IMAGE_EXTENSIONS.contains(&ext.to_str().unwrap()) {
                    comic.pages.push(Page::from_filename(&name, &ext.to_str().unwrap()))
                }
            // Try creating metadata from file
            } else if let Some(metadata) = Metadata::from_metadata_file(&name, &mut file) {
                comic.metadata = metadata;
            }
        }
        return Ok(comic);
    }
}
