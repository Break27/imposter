use clap::Parser;

mod http;
pub mod agent;
pub mod error;
pub mod connection;
pub mod server;


#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    port: Option<u16>,

    #[arg(short, long, value_name = "URL")]
    filter_url: Option<url::Url>,

    #[arg(long, value_name = "SIZE")]
    buf_size: Option<usize>,

    #[arg(short, long, value_name = "SEC")]
    timeout: Option<u64>,

    #[arg(value_name = "URL")]
    remote: url::Url
}

const URL: &str = "https://raw.githubusercontent.com/gfwlist/gfwlist/master/gfwlist.txt";
const LOCALHOST: &str = "127.0.0.1";
const PORT: u16 = 9000;
const BUF_SIZE:usize = 1024;
const TIMEOUT: u64 = 15;

async fn try_launch(agent: Result<agent::Agent, error::BuildError>,
                    server: server::Server) -> Result<(), Box<dyn std::error::Error>>
{
    Ok(server.run(agent?).await?)
}

fn main() {
    if std::env::var("RUST_LOG").ok().is_none() {
        std::env::set_var("RUST_LOG", "info");
    }

    let cli = Cli::parse();
    env_logger::init();
    
    let port = cli.port.unwrap_or(PORT);
    let server = server::Server::bind((LOCALHOST, port));

    let agent = agent::AgentBuilder::new()
        .buffer(cli.buf_size.unwrap_or(BUF_SIZE))
        .filter(cli.filter_url.unwrap_or(URL.parse().unwrap()))
        .timeout(cli.timeout.unwrap_or(TIMEOUT))
        .build(cli.remote);

    if let Err(e) = async_std::task::block_on(
        try_launch(agent, server))
    {
        eprintln!("Error: {}", e);
    }
}
