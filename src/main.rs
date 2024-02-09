#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::redis::{Command, DataType};
use anyhow::{bail, Context, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task,
};

mod redis;

async fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut buf = vec![0u8; 512];

    loop {
        let len = stream
            .read(&mut buf)
            .await
            .context("failed to read stream into buffer")?;
        let command = std::str::from_utf8(&buf[0..len]).context("command not valid utf-8")?;
        debug!("Receiving command: {:?}", command);

        if command != "*1\r\n$4\r\nping\r\n" {
            bail!("can only support ping for now");
        }

        stream
            .write_all("+PONG\r\n".as_bytes())
            .await
            .context("failed to write contents to buffer")?;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    initialize_logging();

    info!("Setting up tcp listener");
    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .context("failed binding tcp listener to adress")?;

    loop {
        let stream = listener.accept().await;
        match stream {
            Ok((stream, _)) => {
                info!("Accepted new connection");
                task::spawn(async {
                    if let Err(e) = handle_connection(stream).await {
                        error!("Connection failed: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept incoming connection: {}", e);
            }
        }
    }
}

fn initialize_logging() {
    use std::io::Write;

    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .target(env_logger::Target::Stdout)
        .format(
            |buf, rec| match (rec.module_path(), rec.file(), rec.line()) {
                (Some(module_path), Some(file), Some(line)) => {
                    writeln!(
                        buf,
                        "[{}][{}]({}:{}) {}",
                        rec.level(),
                        module_path,
                        file,
                        line,
                        rec.args()
                    )
                }
                _ => writeln!(buf, "[{}] {}", rec.level(), rec.args()),
            },
        )
        .filter_module("redis_starter_rust", log::LevelFilter::Trace)
        .init();
}
