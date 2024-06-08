use std::sync::Mutex;
use adblock::request::Request;


pub struct Engine {
    inner: Option<Mutex<adblock::Engine>>
}

impl Engine {
    pub fn new(inner: Option<adblock::Engine>) -> Self {
        Self { inner: inner.map(|x| x.into()) }
    }

    pub fn check_request_blocked(&self, url: &str) -> bool {
        let inner = match &self.inner {
            Some(x) => x.lock().unwrap(),
            None => return true // always use tunnel when without rules
        };

        Request::new(url, url, "fetch")
            .map(|req| inner.check_network_request(&req).matched)
            .unwrap_or(true)
    }
}
