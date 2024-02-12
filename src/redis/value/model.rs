use super::resp::{RespParser, RespSerializer};
use anyhow::Result;
use bstr::BString;

#[derive(Debug, PartialEq, Eq)]
pub enum RespValue {
    SimpleString(String),
    BulkString(BString),
    Array(Box<[RespValue]>),
    Nil,
}

impl RespValue {
    pub fn from_resp(wire: &[u8]) -> Result<Self> {
        RespParser::parse_single(wire)
    }

    pub fn from_resp_all(wire: &[u8]) -> Result<Vec<Self>> {
        RespParser::parse(wire)
    }

    pub fn to_resp(&self) -> std::io::Result<BString> {
        let mut buf = vec![];
        RespSerializer::serialize(&mut buf, self)?;
        Ok(BString::from(buf))
    }

    pub fn write_resp(&self, f: &mut dyn std::io::Write) -> std::io::Result<()> {
        RespSerializer::serialize(f, self)
    }
}
