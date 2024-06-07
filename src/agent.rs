use async_std::io::{Read, Write, ReadExt, WriteExt};
use async_std::net::TcpStream;

use crate::connection::ConnectionBuilder;
use crate::error::{Result, Error, BuildError, BuildResult};
use crate::http;

pub struct AgentBuilder {
    filter_url: Option<url::Url>,
    buf_size: Option<usize>,
    timeout: Option<u64>,
    decode: bool
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self {
            filter_url: None,
            buf_size: None,
            timeout: None,
            decode: true
        }
    }

    pub fn filter(mut self, url: url::Url) -> Self {
        let _ = self.filter_url.insert(url);
        self
    }

    pub fn timeout(mut self, timeout: u64) -> Self {
        let _ = self.timeout.insert(timeout);
        self
    }

    pub fn buffer(mut self, size: usize) -> Self {
        let _ = self.buf_size.insert(size);
        self
    }

    pub fn decode(mut self, decode: bool) -> Self {
        self.decode = decode;
        self
    }

    pub fn build(self, remote: url::Url) -> BuildResult<Agent> {
        use ConnectionBuilder as CB;
        let builder = match remote.scheme() {
            "http"  | ""       => CB::Http(remote.authority().to_string()),
            "socks" | "socks5" => CB::Socks5(remote.authority().to_string()),
            other => return Err(BuildError::Unsupported(other.to_string()))
        };

        let mut ruleset = None;
        let time = self.timeout.unwrap_or(u64::MAX);

        let config = AgentConfig {
            buf_size: self.buf_size.unwrap_or(1024),
            timeout: std::time::Duration::from_secs(time)
        };

        if let Some(ref url) = self.filter_url {
            log::info!(target: "builder", "Try downloading rule list from '{url}'");
            let https = native_tls::TlsConnector::new()?;

            let client = ureq::AgentBuilder::new()
                .proxy(ureq::Proxy::new(remote)?)
                .tls_connector(https.into())
                .timeout(config.timeout)
                .build();
            let resp = client.get(url.as_str()).call()?;
            let text = resp.into_string()?;
            let len = text.len() as f32 / 1000f32;

            log::info!(target: "builder", "Successfully downloaded data ({len}/kB transmitted)");
            ruleset = Some(self.build_rules(text)?);
        }

        Ok(Agent { builder, ruleset, config })
    }

    fn build_rules(&self, mut text: String) -> BuildResult<adblock::Engine> {
        if self.decode {
            log::info!(target: "builder", "Try decoding raw textual data (base64 encoded)");
            use base64::{Engine, engine::general_purpose::STANDARD};
            let line = text.split_whitespace().collect::<String>();
            let decoded = STANDARD.decode(line)?;

            text = String::from_utf8(decoded)?;
        }

        let mut filters = adblock::FilterSet::new(false);
        let opts = adblock::lists::ParseOptions::default();
        filters.add_filter_list(&text, opts);

        log::info!(target: "builder", "Rule data parsed successfully");
        Ok(adblock::Engine::from_filter_set(filters, true))
    }
}

pub struct AgentConfig {
    pub buf_size: usize,
    pub timeout: std::time::Duration,
}

pub struct Agent {
    ruleset: Option<adblock::Engine>,
    builder: ConnectionBuilder,
    config: AgentConfig,
}

unsafe impl Send for Agent {}
unsafe impl Sync for Agent {}

impl Agent {
    pub async fn handle<S>(&self, mut conn: S) -> Result<()>
    where
        S: Read + Write + Send + Sync + Unpin + 'static
    {
        let request = self.read(&mut conn)?;
        let host = request.host();

        log::info!("CLIENT --> {host}");

        if self.check_request_blocked(&request.path) {
            log::info!("CLIENT --> PROXY --> {host}");
            let mut outbound = self.io(self.builder.connect(&host))?;

            // forward intercepted request
            outbound.write_all(request.as_bytes()).await?;
            outbound.flush().await?;

            log::info!("CLIENT <-> PROXY (connection established)");
            self.tunnel(conn, outbound).await?;
            return Ok(());
        }

        let target = self.io(TcpStream::connect(host))?;
        log::info!("CLIENT <-> TARGET (direct)");

        if let http::Method::CONNECT = request.method {
            let resp = b"HTTP/1.1 200 OK\r\n\r\n";
            // send response to client with code 200 and an EMPTY body
            conn.write_all(resp).await?;
            conn.flush().await?;
            log::debug!("Received CONNECT (200 OK)");
        }

        self.tunnel(conn, target).await?;
        return Ok(());
    }

    async fn tunnel<A, B>(&self, mut inbound: A, mut outbound: B) -> Result<()>
    where
        A: Read + Write + Send + Sync + Unpin + 'static,
        B: Read + Write + Send + Sync + Unpin + 'static,
    {
        use async_compat::CompatExt;
        use tokio::io::copy_bidirectional as copy;

        if let Err(e) = copy(
            &mut outbound.compat_mut(), &mut inbound.compat_mut()).await
        {
            log::warn!("{}", e);
        }

        Ok(())
    }

    fn read<S>(&self, conn: &mut S) -> Result<http::Request>
    where
        S: Read + Write + Send + Unpin + 'static
    {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut request = httparse::Request::new(&mut headers);

        let mut buf = vec![0; self.config.buf_size];
        self.io(conn.read(&mut buf))?;

        let offset = request.parse(&buf)?.unwrap();
        let payload = buf[..offset].to_vec();

        let method = match request.method {
            Some(x) => x.parse::<crate::http::Method>().unwrap(),
            None => return Err(Error::BadRequest("METHOD".to_string()))
        };

        let mut path = match request.path {
            Some(x) => x.to_string(),
            None => return Err(Error::BadRequest("PATH".to_string()))
        };

        if path.find("://").is_none() {
            // in case of an cannot-be-a-base url
            // find a port number, if any
            let port = path
                .rfind(":")
                .and_then(|x| path.get(x + 1..));

            let scheme = match port {
                Some("443") => "https",
                Some("21") => "ftp",
                Some("80") | _ => "http",
            };

            path = format!("{}://{}", scheme, path);
        }

        let version = match request.version {
            Some(3)  => http::Version::HTTP_3,
            Some(2)  => http::Version::HTTP_2,
            Some(11) => http::Version::HTTP_11,
            Some(1)  => http::Version::HTTP_10,
            Some(_)  => http::Version::HTTP_09,
            None => return Err(Error::BadRequest("VERSION".to_string()))
        };

        let mut host = headers.iter()
            .find_map(|x: _| (x.name == "Host").then_some(x.value))
            .map(|x| std::str::from_utf8(x))
            .ok_or(Error::BadRequest("Host".to_string()))??
            .to_string();

        if host.find(":").is_none() {
            // append a port number when without one
            host += ":80";
        }

        let request = crate::http::Request {
            method,
            path,
            version,
            host,
            payload: payload.into(),
        };

        Ok(request)
    }

    fn check_request_blocked(&self, url: &str) -> bool {
        let attempt: _ = adblock::request::Request::new(
            url, url, "fetch"
        );

        let req = match attempt {
            Ok(x) => x,
            Err(_) => return true
        };

        match &self.ruleset {
            Some(x) => x.check_network_request(&req).matched,
            None => true // always use tunnel when without rules
        }
    }

    fn io<T, F>(&self, f: F) -> Result<T>
    where
        F: std::future::Future<Output=std::result::Result<T, std::io::Error>>,
    {
        async_std::task::block_on(async {
            Ok(async_std::io::timeout(self.config.timeout, f).await?)
        })
    }
}
