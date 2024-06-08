use adblock::request::Request;


pub struct Engine {
    inner: Option<adblock::Engine>
}

impl Engine {
    pub fn new(inner: Option<adblock::Engine>) -> Self {
        Self { inner }
    }

    pub fn check_request_blocked(&self, url: &str) -> bool {
        let inner = match &self.inner {
            Some(x) => x,
            None => return true // always use tunnel when without rules
        };

        Request::new(url, url, "fetch")
            .map(|x| inner.check_network_request(&x).matched)
            .unwrap_or(true)
    }
}
