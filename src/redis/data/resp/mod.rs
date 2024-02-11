use super::{error, model};

pub(in crate::redis) use parser::RespParser;

mod parser;

const CRLF: &str = "\r\n";
