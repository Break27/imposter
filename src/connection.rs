use std::net::TcpStream;
use std::pin::Pin;

use async_io::Async;
use socks::Socks5Stream;

pub enum ConnectionBuilder {
    Http(String),
    Socks5(String)
}

impl ConnectionBuilder {
    pub async fn connect(&self, target: &str) -> std::io::Result<Connection> {
        let conn = match self {
            Self::Http(addr) => {
                Connection::new(TcpStream::connect(addr)?)
            },
            Self::Socks5(addr) => {
                Connection::new(Socks5Stream::connect(addr, target)?.into_inner())
            }
        };
        Ok(conn)
    }
}

pub struct Connection {
    inner: Async<TcpStream>
}

unsafe impl Send for Connection {}
unsafe impl Sync for Connection {}

impl Connection {
    pub fn new(conn: TcpStream) -> Self 
    {
        Self { inner: Async::new(conn).unwrap() }
    }

    pub fn into_inner(self) -> std::io::Result<TcpStream> {
        self.inner.into_inner()
    }

    pub fn shutdown(self, how: std::net::Shutdown) -> std::io::Result<()> {
        self.into_inner()?.shutdown(how)
    }
}

impl async_std::io::Read for Connection {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        ctx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        Async::poll_read(Pin::new(&mut self.inner), ctx, buf)
    }
}

impl async_std::io::Write for Connection {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        ctx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        Async::poll_write(Pin::new(&mut self.inner), ctx, buf)
    }

    fn poll_flush(mut self: std::pin::Pin<&mut Self>, ctx: &mut std::task::Context<'_>)
        -> std::task::Poll<std::io::Result<()>>
    {
        Async::poll_flush(Pin::new(&mut self.inner), ctx)
    }

    fn poll_close(mut self: std::pin::Pin<&mut Self>, ctx: &mut std::task::Context<'_>)
        -> std::task::Poll<std::io::Result<()>>
    {
        Async::poll_close(Pin::new(&mut self.inner), ctx)
    }
}
