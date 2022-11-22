use clap::Parser;
use mail_to_telegram::{error::Error, server::SMTPTelegramServerBuilder};
use simple_logger::SimpleLogger;
use std::{env, net::SocketAddr, cmp::min};
use tokio::runtime::Builder;

/// SMTP Server that forwards all emails as telegram messages
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Telegram Bot API Token, if not present it will try to use get it from the TELEGRAM_BOT_TOKEN environment var
    #[arg(short, long)]
    api_token: Option<String>,

    #[arg(short, long)]
    /// How many threads should the async-std runtime use, can also be set with the ASYNC_STD_THREAD_COUNT variable, default is one thread per logical cpu
    thread_count: Option<usize>,

    #[arg(short, long)]
    /// If chat id can't be parsed out of the recipient email (use following format: YOUR_CHAT_ID@telegram-bot.com), it can fall back to the chat_id. Alternatively you can use the TELEGRAM_STANDARD_CHAT_ID environment variable. If no standard is present and the chat id can't be parsed, no telegram message is sent
    standard_chat_id: Option<u64>,

    /// The host the server is going to be hosted on. Default is 0.0.0.0:17333
    #[arg(long)]
    host: Option<SocketAddr>,
}

async fn create_server(args: Args) -> Result<(), Error> {
    let api_token = args.api_token.or_else(|| env::var("TELEGRAM_BOT_TOKEN").ok()).expect("No Telegram Bot Api Token supplied with the command line options or the TELEGRAM_BOT_TOKEN environment variable");
    let mut standard_chat_id = args.standard_chat_id;
    if standard_chat_id.is_none() {
        let id = env::var("TELEGRAM_STANDARD_CHAT_ID").map(|id| {
            id.parse::<u64>()
                .unwrap_or_else(|_| panic!("ID '{}' could not be parsed to a number", id))
        });
        if let Ok(id) = id {
            standard_chat_id = Some(id);
        }
    }
    let server = SMTPTelegramServerBuilder::new(api_token)
        .with_socket(args.host)
        .with_standard_chat_id(standard_chat_id)
        .build()
        .await?;
    server.listen().await
}

fn main() -> Result<(), Error> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .init()?;
    let args = Args::parse();
    let thread_count: usize;
    if let Some(tc) = args.thread_count.or_else(|| {
        env::var("ASYNC_STD_THREAD_COUNT")
            .map(|tc| tc.parse::<usize>().ok())
            .ok()
            .flatten()
    }) {
        thread_count = tc;
    } else {
        thread_count = min(num_cpus::get(), 4);
    }

    let runtime = Builder::new_multi_thread()
        .worker_threads(thread_count)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(create_server(args))
}
