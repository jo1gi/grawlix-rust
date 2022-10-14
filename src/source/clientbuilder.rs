use std::collections::HashMap;

/// Builder for reqwest client
#[derive(Default)]
pub struct ClientBuilder {
    headers: Vec<(String, String)>,
    cookies: Vec<(String, String)>,
}


impl ClientBuilder {

    pub fn cookie<S: ToString>(mut self, key: S, value: S) -> Self {
        self.add_cookie(key, value);
        self
    }

    pub fn add_cookie<S: ToString>(&mut self, key: S, value: S) {
        self.cookies.push((key.to_string(), value.to_string()))
    }

    pub fn header<S: ToString>(mut self, key: S, value: S) -> Self {
        self.add_header(key, value);
        self
    }

    pub fn add_header<S: ToString>(&mut self, key: S, value: S) {
        self.headers.push((key.to_string(), value.to_string()))
    }

    pub fn to_reqwest_client(&self) -> reqwest::Client {
        let reqwest_builder = reqwest::Client::builder();
        let mut headers = create_reqwest_headermap(&self.headers);
        headers.insert(
            reqwest::header::COOKIE,
            // TODO: Remove unwrap
            create_cookie_string(&self.cookies).parse().unwrap()
        );
        reqwest_builder
            .default_headers(headers)
            .build()
            // TODO: Remove unwrap
            .unwrap()
    }
}

fn create_cookie_string(cookies: &Vec<(String, String)>) -> String {
    cookies.iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<String>>()
        .join("; ")
}

fn create_reqwest_headermap(headers: &Vec<(String, String)>) -> reqwest::header::HeaderMap {
    let mut hashmap = HashMap::new();
    for (key, value) in headers {
        hashmap.insert(key.clone(), value.clone());
    }
    // TODO: Remove unwrap
    reqwest::header::HeaderMap::try_from(&hashmap).unwrap()
}
