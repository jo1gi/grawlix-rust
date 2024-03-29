use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crypto::{
    aes::{KeySize, cbc_decryptor},
    blockmodes::NoPadding,
    buffer::{RefReadBuffer, RefWriteBuffer, WriteBuffer, ReadBuffer},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct Page {
    pub file_format: String,
    pub page_type: PageType,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PageType {
    /// Page on website
    Url(OnlinePage),
    /// Page in container
    Container(String),
}

/// Instructions on how to download a page
#[derive(Default, Debug, Deserialize, Serialize)]
pub struct OnlinePage {
    /// Url of page
    pub url: String,
    /// Required headers for request
    pub headers: Option<HashMap<String, String>>,
    /// Encryption scheme of page
    pub encryption: Option<PageEncryptionScheme>
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PageEncryptionScheme {
    /// AES encryption
    AES {
        key: Vec<u8>,
        iv: Vec<u8>,
    },
    /// Encryption scheme used by DC Universe Infinite
    DCUniverseInfinite([u8; 32]),
    /// XOR encryption
    XOR(Vec<u8>),
}

impl Page {
    pub fn from_url(url: &str, file_format: &str) -> Self {
        Self {
            file_format: file_format.to_string(),
            page_type: PageType::Url(OnlinePage {
                url: url.to_string(),
                ..Default::default()
            })
        }
    }

    pub fn from_url_with_headers(url: &str, headers: HashMap<String, String>, file_format: &str) -> Self {
        Self {
            file_format: file_format.to_string(),
            page_type: PageType::Url(OnlinePage {
                url: url.to_string(),
                headers: Some(headers),
                encryption: None,
            })
        }
    }

    pub fn from_url_xor(url: &str, key: Vec<u8>, file_format: &str) -> Self {
        Self {
            file_format: file_format.to_string(),
            page_type: PageType::Url(OnlinePage {
                url: url.to_string(),
                headers: None,
                encryption: Some(PageEncryptionScheme::XOR(key))
            })
        }
    }

    pub fn from_filename(filename: &str, file_format: &str) -> Self {
        Self {
            file_format: file_format.to_string(),
            page_type: PageType::Container(filename.to_string())
        }
    }
}

impl OnlinePage {
    pub async fn download_page(&self, client: &reqwest::Client) -> Vec<u8> {
        log::trace!("Downloading page: {}", self.url);
        let mut req = client.get(&self.url);
        if let Some(headers) = &self.headers {
            req = req.headers(headers.try_into().unwrap());
        }
        // TODO: Remove unwraps
        let resp = req.send().await.unwrap();
        let bytes = resp.bytes().await.unwrap().as_ref().into();
        match &self.encryption {
            Some(enc) => decrypt_page(bytes, enc),
            None => bytes
        }
    }
}

fn decrypt_page(bytes: Vec<u8>, enc: &PageEncryptionScheme) -> Vec<u8> {
    log::trace!("Decrypting page");
    match enc {
        PageEncryptionScheme::AES { key, iv } => {
            let mut image_buffer = RefReadBuffer::new(&bytes);
            let size = bytes.len();
            let mut decrypted_vector = vec![0; size];
            let mut decrypted_buffer = RefWriteBuffer::new(&mut decrypted_vector);
            let mut aescbc = cbc_decryptor(KeySize::KeySize128, key, iv, NoPadding);
            aescbc.decrypt(&mut image_buffer, &mut decrypted_buffer, true)
                // TODO: Handle correct
                .expect("Could not decrypt image with AES");
            // Gets image data
            let mut image = decrypted_buffer.take_read_buffer();
            image.take_remaining().to_vec()
        },
        PageEncryptionScheme::XOR(key) => {
            bytes.iter()
                .zip(key.iter().cycle())
                .map(|(v, k)| v ^ k)
                .collect()
        },
        PageEncryptionScheme::DCUniverseInfinite(key) => {
            // The first 8 bytes contains the size of the output file
            let original_size = &bytes[0..8];
            // Convert the size to a number
            let size = {
                let mut tmp = [0u8; 8];
                tmp.clone_from_slice(&original_size);
                u64::from_le_bytes(tmp) as usize
            };
            // Check if size is correct
            if size > bytes.len() {
                // TODO: Better error handling
                panic!("Size not correct for final image");
            }
            // The next 16 bytes are the initialization vector
            let iv = &bytes[8..24];
            // The rest of the data is the image
            let mut image_buffer = RefReadBuffer::new(&bytes[24..]);
            // Decrypts the image
            let mut decrypted_vector = vec![0; size];
            let mut decrypted_buffer = RefWriteBuffer::new(&mut decrypted_vector);
            let mut aescbc = cbc_decryptor(KeySize::KeySize256, key, iv, NoPadding);
            aescbc.decrypt(&mut image_buffer, &mut decrypted_buffer, true)
                // TODO: Handle correct
                .expect("Could not decrypt image from DC Universe Infinite");
            // Gets image data
            let mut image = decrypted_buffer.take_read_buffer();
            image.take_remaining().to_vec()
        }
    }
}
