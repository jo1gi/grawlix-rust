pub struct HttpRequest {
    method: RequestMethod,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

enum RequestMethod {
    Get,
    Post
}

fn new(url: &str, method: RequestMethod) -> HttpRequest {
    HttpRequest {
        method,
        url: url.to_string(),
        headers: Vec::new(),
        body: None,
    }
}

impl HttpRequest {


    /// Create http GET request
    pub fn get(url: &str) -> Self {
        new(url, RequestMethod::Get)
    }

    /// Create http GET request
    pub fn post(url: &str) -> Self {
        new(url, RequestMethod::Post)
    }

    /// Add header to request
    pub fn header(mut self, key: String, value: String) -> Self {
        self.headers.push((key, value));
        self
    }

    pub fn to_reqwest_request(&self, client: &reqwest::Client) -> reqwest::RequestBuilder {
        let mut request = match self.method {
            RequestMethod::Get => client.get(&self.url),
            RequestMethod::Post => client.post(&self.url)
        };
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }
        request
    }

}
