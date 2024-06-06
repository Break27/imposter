use std::sync::Arc;


pub struct Server {
    addrs: std::net::SocketAddr,
}

impl Server {
    pub async fn run(self, agent: crate::agent::Agent) -> Result<(), std::io::Error> {
        let listener = async_std::net::TcpListener::bind(self.addrs).await?;
        let agent = Arc::new(agent);

        log::info!("IMPOSTER/0.1 HTTP SERVER");
        log::info!("Server listening at {}", self.addrs);

        loop {
            let agent = agent.clone();
            let (inbound, addr) = listener.accept().await?;

            log::info!("*** Incoming connection from {}", addr);

            async_std::task::spawn(async move {
                if let Err(e) = agent.handle(inbound).await {
                    log::error!("Agent: {}", e);
                }
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
