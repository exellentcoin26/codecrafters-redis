use anyhow::{Context, Result};
use log::{debug, error, info, trace, warn};
use std::{
    io::{Read, Write},
    net::TcpListener,
};

fn main() -> Result<()> {
    // initialize logging
    initialize_logging();

    trace!("setting up tcp listener");
    let listener =
        TcpListener::bind("127.0.0.1:6379").context("failed binding tcp listener to adress")?;

    let mut buf = vec![0u8; 512];

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                info!("accepted new connection");
                let len = stream.read(&mut buf)?;
                let command =
                    std::str::from_utf8(&buf[0..len]).context("command not valid utf-8")?;

                assert_eq!(&command, &"*1\r\n$4\r\nping\r\n");

                stream.write_all("+PONG\r\n".as_bytes())?;
            }
            Err(e) => {
                info!("error: {}", e);
            }
        }
    }

    Ok(())
}

fn initialize_logging() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
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
