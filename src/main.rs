use log::{debug, error, info, trace, warn};
use std::net::TcpListener;

fn main() {
    // initialize logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .target(env_logger::Target::Stdout)
        .init();

    trace!("setting up tcp listener");
    let listener =
        TcpListener::bind("127.0.0.1:6379").expect("failed binding tcp listener to adress");

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                info!("accepted new connection");
            }
            Err(e) => {
                info!("error: {}", e);
            }
        }
    }
}
