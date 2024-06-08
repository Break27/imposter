use async_std::net::TcpListener;

use crate::agent::{Agent, AgentConfig};
use crate::connection::ConnectionBuilder;
use crate::error::{BuildError, BuildResult};
use crate::engine::Engine;


#[derive(Default)]
pub struct ServerBuilder {
    filter_url: Option<url::Url>,
    buf_size: Option<usize>,
    timeout: Option<u64>,
    encoded: bool
}

impl ServerBuilder {
    pub fn filter(mut self, url: url::Url) -> Self {
        let _ = self.filter_url.insert(url);
        self
    }

    pub fn buffer(mut self, size: usize) -> Self {
        let _ = self.buf_size.insert(size);
        self
    }

    pub fn timeout(mut self, timeout: u64) -> Self {
        let _ = self.timeout.insert(timeout);
        self
    }

    pub fn encoded(mut self, encoded: bool) -> Self {
        self.encoded = encoded;
        self
    }

    pub fn build(self, remote: url::Url) -> BuildResult<Server> {
        use ConnectionBuilder as CB;
        let builder = match remote.scheme() {
            "http"  | ""       => CB::Http(remote.authority().to_string()),
            "socks" | "socks5" => CB::Socks5(remote.authority().to_string()),
            other => return Err(BuildError::Unsupported(other.to_string()))
        };

        let mut ruleset = None;
        let time = self.timeout.unwrap_or(u64::MAX);

        let config = AgentConfig {
            bufsize: self.buf_size.unwrap_or(1024),
            timeout: std::time::Duration::from_secs(time),
        };

        if let Some(ref url) = self.filter_url {
            log::info!("Try downloading rule list from '{url}'");
            let https = native_tls::TlsConnector::new()?;

            let client = ureq::AgentBuilder::new()
                .proxy(ureq::Proxy::new(remote)?)
                .tls_connector(https.into())
                .timeout(config.timeout)
                .build();
            let resp = client.get(url.as_str()).call()?;
            let text = resp.into_string()?;
            let len = text.len() as f32 / 1000f32;

            log::info!("Successfully downloaded data ({len}/kB transmitted)");
            ruleset = Some(self.build_rules(text)?);
        }

        let engine = Engine::new(ruleset);
        let agent = Agent::new(builder, config, engine);

        Ok(Server { agent: agent.into() })
    }

    fn build_rules(&self, mut text: String) -> BuildResult<adblock::Engine> {
        if self.encoded {
            log::info!("Try decoding raw textual data (base64 encoded)");
            use base64::{Engine, engine::general_purpose::STANDARD};
            let line = text.split_whitespace().collect::<String>();
            let decoded = STANDARD.decode(line)?;

            text = String::from_utf8(decoded)?;
        }

        let mut filters = adblock::FilterSet::new(false);
        let opts = adblock::lists::ParseOptions::default();
        filters.add_filter_list(&text, opts);

        log::info!("Rule data parsed successfully");
        Ok(adblock::Engine::from_filter_set(filters, true))
    }
}

pub struct Server {
    agent: std::sync::Arc<Agent>
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::default()
    }

    pub async fn bind<A>(self, addrs: A) -> std::io::Result<()>
    where
        A: std::net::ToSocketAddrs
    {
        let addrs = addrs.to_socket_addrs()?
            .collect::<Vec<std::net::SocketAddr>>()
            .pop()
            .expect("Bind Error");

        log::info!("IMPOSTER/0.1 HTTP SERVER");
        log::info!("Server listening at {addrs}");

        let listener = TcpListener::bind(addrs).await?;
        loop {
            let (inbound, addr) = listener.accept().await?;
            let agent = self.agent.clone();

            log::info!("*** Incoming connection from {addr}");

            async_std::task::spawn(async move {
                agent.handle(inbound).await;
            });
        }
    }
}
