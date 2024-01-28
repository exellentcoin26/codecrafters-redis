#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use anyhow::{Context, Result};
use tokio::{
    net::{TcpListener, TcpStream},
    task,
};

async fn handle_connection(stream: TcpStream) -> Result<()> {
    stream.readable().await?;

    let mut buf = vec![0u8; 512];
    let len = stream
        .try_read(&mut buf)
        .context("failed to read stream contents into buffer")?;
    let command = std::str::from_utf8(&buf[0..len]).context("command not valid utf-8")?;
    assert_eq!(&command, &"*1\r\n$4\r\nping\r\n");

    stream.writable().await?;
    stream.try_write("+PONG\r\n".as_bytes())?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // initialize logging
    initialize_logging();

    debug!("setting up tcp listener");
    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .context("failed binding tcp listener to adress")?;

    loop {
        let stream = listener.accept().await;
        match stream {
            Ok((stream, _)) => {
                debug!("accepted new connection");
                task::spawn(handle_connection(stream));
            }
            Err(e) => {
                error!("failed to accept incoming connection: {}", e);
            }
        }
    }
}

fn initialize_logging() {
    use std::io::Write;

    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
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
        .init();
}
