use async_std::io::{Read, ReadExt, Write, WriteExt};
use async_std::net::TcpStream;

use crate::connection::ConnectionBuilder as Builder;
use crate::engine::Engine;
use crate::error::{Error, Result};
use crate::http;

pub struct AgentConfig {
    pub bufsize: usize,
    pub timeout: std::time::Duration,
}

pub struct Agent {
    builder: Builder,
    config: AgentConfig,
    engine: Engine
}

unsafe impl Send for Agent {}
unsafe impl Sync for Agent {}

impl Agent {
    pub fn new(builder: Builder, config: AgentConfig, engine: Engine) -> Self {
        Self { builder, config, engine }
    }

    pub async fn handle(&self, mut conn: TcpStream) {
        let req = match self.read(&mut conn) {
            Ok(x) => x,
            Err(e) => return log::error!("Read: {e}")
        };

        let mat = self.engine.check_request_blocked(&req.path);

        log::info!("CLIENT --> {} ({})",
            req.host, mat.then_some("tunnel").unwrap_or("direct"));

        let res = if mat {
            self.remote(req, &mut conn).await
        } else {
            self.direct(req, &mut conn).await
        };

        if let Err(e) = res {
            log::error!("Agent: {e}");
            let resp = http::Response::from_err(e);

            conn.write(resp.to_string().as_bytes()).await.unwrap();
            conn.flush().await.unwrap();
        }

        let _ = conn.shutdown(std::net::Shutdown::Both);
    }

    async fn remote<S>(&self, req: http::Request, inbound: &mut S) -> Result<()>
    where
        S: Read + Write + Send + Sync + Unpin + 'static
    {
        let mut outbound = self.io(self.builder.connect(&req.host))?;
        log::info!("CLIENT --> PROXY (pending)");

        // forward intercepted request
        outbound.write_all(req.as_bytes()).await?;
        outbound.flush().await?;

        log::info!("CLIENT <=> PROXY (connection established)");
        self.tunnel(inbound, &mut outbound).await;

        let _ = outbound.shutdown(std::net::Shutdown::Both);
        return Ok(());
    }

    async fn direct<S>(&self, req: http::Request, inbound: &mut S) -> Result<()>
    where
        S: Read + Write + Send + Sync + Unpin + 'static
    {
        let mut outbound = self.io(TcpStream::connect(&req.host))?;
        log::info!("CLIENT --> TARGET (pending)");

        if let http::Method::CONNECT = req.method {
            let resp = http::Response::default();
            // respond to client with code 200 and an EMPTY body
            inbound.write_all(resp.to_string().as_bytes()).await?;
            inbound.flush().await?;
            log::debug!("Agent: received CONNECT (200 OK)");
        } else {
            // forward intercepted request
            outbound.write_all(req.as_bytes()).await?;
            outbound.flush().await?;
            log::debug!("CLIENT --> (intercepted) --> TARGET");
        }

        log::info!("CLIENT <=> TARGET (connection established)");
        self.tunnel(inbound, &mut outbound).await;
        let _ = outbound.shutdown(std::net::Shutdown::Both);

        return Ok(());
    }

    async fn tunnel<A, B>(&self, inbound: &mut A, outbound: &mut B)
    where
        A: Read + Write + Send + Sync + Unpin + 'static,
        B: Read + Write + Send + Sync + Unpin + 'static,
    {
        use async_compat::CompatExt;
        use tokio::io::copy_bidirectional as copy;

        if let Err(e) = copy(
            &mut outbound.compat_mut(), &mut inbound.compat_mut()).await
        {
            log::warn!("{e}");
        }
    }

    fn read(&self, conn: &mut TcpStream) -> Result<http::Request> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut request = httparse::Request::new(&mut headers);

        let mut buf = vec![0; self.config.bufsize];
        self.io(conn.read(&mut buf))?;

        let offset = request.parse(&buf)?.unwrap();
        let payload = buf[..offset].to_vec();

        let method = match request.method {
            Some(x) => x.parse::<http::Method>().unwrap(),
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

            path = format!("{scheme}://{path}");
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

        let request = http::Request {
            method,
            path,
            version,
            host,
            payload: payload.into(),
        };

        Ok(request)
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
