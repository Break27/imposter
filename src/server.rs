pub struct Server {
    addrs: std::net::SocketAddr,
}

impl Server {
    pub async fn run(self, agent: crate::agent::Agent) -> std::io::Result<()> {
        let listener = async_std::net::TcpListener::bind(self.addrs).await?;
        let agent = std::sync::Arc::new(agent);

        log::info!("IMPOSTER/0.1 HTTP SERVER");
        log::info!("Server listening at {}", self.addrs);

        loop {
            let agent = agent.clone();
            let (mut inbound, addr) = listener.accept().await?;

            log::info!("*** Incoming connection from {addr}");

            async_std::task::spawn(async move {
                if let Err(e) = agent.handle(&mut inbound).await {
                    log::error!("Agent: {e}");

                    let resp = crate::http::Response::from_err(e);
                    use async_std::io::WriteExt;
                    
                    inbound.write(resp.to_string().as_bytes()).await.unwrap();
                    inbound.flush().await.unwrap();
                }

                let _ = inbound.shutdown(std::net::Shutdown::Both);
            });
        }
    }

    pub fn bind<A>(addrs: A) -> Self
    where
        A: std::net::ToSocketAddrs
    {
        let addrs = addrs.to_socket_addrs()
             .expect("Bind Error")
             .collect::<Vec<std::net::SocketAddr>>()
             .pop()
             .expect("Bind Error");

        Self { addrs }
    }
}
