use super::{error, model};

pub(in crate::redis) use parser::RespParser;
pub(in crate::redis) use serializer::RespSerializer;

mod parser;
mod serializer;

const CRLF: &str = "\r\n";
