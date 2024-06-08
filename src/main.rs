use std::ops::Not;
use clap::Parser;

pub mod http;
pub mod agent;
pub mod error;
pub mod connection;
pub mod server;
pub mod engine;


#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Server port number
    #[arg(short, long)]
    port: Option<u16>,

    /// Rule data URL
    #[arg(short, long, value_name = "URL")]
    filter_url: Option<url::Url>,

    /// Buffer size
    #[arg(long, value_name = "SIZE")]
    buf_size: Option<usize>,

    /// Seconds before server return a timeout response (408)
    #[arg(short, long, value_name = "SEC")]
    timeout: Option<u64>,

    /// Load downloaded rule data without decoding (base64)
    #[arg(long)]
    plain_text: bool,

    /// Proxy server URL
    #[arg(value_name = "URL")]
    remote: url::Url
}

const URL: &str = "https://raw.githubusercontent.com/gfwlist/gfwlist/master/gfwlist.txt";
const LOCALHOST: &str = "127.0.0.1";
const PORT: u16 = 9000;
const BUF_SIZE:usize = 1024;
const TIMEOUT: u64 = 15;

async fn launch() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RUST_LOG").ok().is_none() {
        unsafe { std::env::set_var("RUST_LOG", "info") }
    }

    let cli = Cli::parse();
    env_logger::init();
    
    let port = cli.port.unwrap_or(PORT);
    let server: _ = server::Server::builder()
        .buffer(cli.buf_size.unwrap_or(BUF_SIZE))
        .filter(cli.filter_url.unwrap_or(URL.parse().unwrap()))
        .timeout(cli.timeout.unwrap_or(TIMEOUT))
        .encoded(cli.plain_text.not())
        .build(cli.remote)?;

    Ok(server.bind((LOCALHOST, port)).await?)
}

fn main() {
    if let Err(e) =
        async_std::task::block_on(launch())
    {
        eprintln!("Error: {e}");
    }
}
