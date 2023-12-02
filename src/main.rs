use anyhow::Context;
use clap::{Arg, Command};
use futures_util::{StreamExt, TryStreamExt};
use log::info;
use std::{future, io::Write};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");

    let app = Command::new("broadcast")
        .version("0.1.0")
        .author("nop")
        .about("WebRTC")
        .subcommand_negates_reqs(true)
        .arg(
            Arg::new("debug")
                .long("debug")
                .short('d')
                .help("Prints debug log")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("addr")
                .long("addr")
                .short('a')
                .help("Address to listen")
                .default_value("127.0.0.1:8080"),
        );

    let matches = app.clone().get_matches();

    if matches.get_flag("debug") {
        println!("Debug mode");
        env_logger::Builder::new()
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{}:{} [{}] {} - {}",
                    record.file().unwrap_or("unknown"),
                    record.line().unwrap_or(0),
                    record.level(),
                    chrono::Local::now().format("%H:%M:%S.%6f"),
                    record.args()
                )
            })
            .filter(None, log::LevelFilter::Trace)
            .init();
    }

    let addr = matches
        .get_one::<String>("addr")
        .context("Failed to get addr")?;

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }

    Ok(())
}

async fn accept_connection(stream: TcpStream) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    info!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (write, read) = ws_stream.split();
    // We should not forward messages other than text or binary.
    read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
        .forward(write)
        .await
        .expect("Failed to forward messages")
}
