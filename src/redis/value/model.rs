use super::resp::{RespParser, RespSerializer};
use anyhow::Result;
use bstr::BString;

#[derive(Debug, PartialEq, Eq)]
pub enum RespValue {
    SimpleString(String),
    BulkString(BString),
    Array(Box<[RespValue]>),
}

impl RespValue {
    pub fn from_resp(wire: &[u8]) -> Result<Self> {
        RespParser::parse_single(wire)
    }

    pub fn from_resp_all(wire: &[u8]) -> Result<Vec<Self>> {
        RespParser::parse(wire)
    }

    pub fn write_resp(&self, f: &mut dyn std::io::Write) -> std::io::Result<()> {
        RespSerializer::serialize(f, self)
    }
}
