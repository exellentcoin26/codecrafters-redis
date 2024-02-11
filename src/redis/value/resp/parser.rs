use super::{error::Error, model::RespValue, CRLF};
use anyhow::{bail, Context, Result};
use bstr::{BString, ByteSlice};

pub(in crate::redis) struct RespParser;

impl RespParser {
    pub(in crate::redis) fn parse(wire: &[u8]) -> Result<Vec<RespValue>> {
        let mut wire = wire.split_str(CRLF).peekable();
        let mut result = Vec::new();
        while wire.peek().is_some() {
            result.push(Self::parse_resp(&mut wire)?)
        }
        Ok(result)
    }

    pub(in crate::redis) fn parse_single(wire: &[u8]) -> Result<RespValue> {
        let mut wire = wire.split_str(CRLF);
        let result = Self::parse_resp(&mut wire)?;
        match wire.find(|&l| !matches!(l, b"")) {
            None => Ok(result),
            Some(_) => bail!("wire is not empty after parsing a single resp value"),
        }
    }

    fn parse_resp<'a>(lines: &mut impl Iterator<Item = &'a [u8]>) -> Result<RespValue> {
        let mut first_line = lines.next().context(Error::EmptyInput)?.iter();
        let Some(first_byte) = first_line.next() else {
            bail!("resp line must have a first byte indicating the datatype");
        };

        Ok(match first_byte {
            b'+' => Self::parse_simple_string(first_line.as_slice())?,
            b'*' => Self::parse_array(Self::parse_u32(first_line.as_slice())?, lines)?,
            b'$' => Self::parse_bulk_string(
                Self::parse_u32(first_line.as_slice())?,
                lines
                    .next()
                    .context("missing second line for bulk string data")?,
            )?,
            _ => bail!("unimplemented datatype `{:?}`", first_byte),
        })
    }

    fn parse_u32(value: &[u8]) -> Result<u32> {
        std::str::from_utf8(value)
            .context(Error::InvalidUtf8)?
            .parse::<u32>()
            .context("failed to parse value as 32 bit unsigned integer")
    }

    fn parse_simple_string(value: &'_ [u8]) -> Result<RespValue> {
        if value.contains(&b'\n') || value.contains(&b'\r') {
            bail!(r"input cannot contain `\r` or `\n`");
        }
        Ok(RespValue::SimpleString(String::try_from(value.as_bstr())?))
    }

    fn parse_array<'b>(
        length: u32,
        lines: &mut impl Iterator<Item = &'b [u8]>,
    ) -> Result<RespValue> {
        Ok(RespValue::Array(
            (0..length)
                .map(|_| Self::parse_resp(&mut *lines))
                .collect::<Result<_>>()?,
        ))
    }

    fn parse_bulk_string(length: u32, value: &'_ [u8]) -> Result<RespValue> {
        if value.len() != length as usize {
            bail!("bulk string value size is not equal to the expected size (expected: `{}`, got: `{}`)", value.len(), length);
        }

        Ok(RespValue::BulkString(BString::from(value)))
    }
}

#[cfg(test)]
mod tests {
    use super::{RespParser, RespValue};
    use bstr::BString;

    #[test]
    fn parse_simple_string() {
        let simple_string = RespParser::parse_single(b"+PONG\r\n").unwrap();
        assert_eq!(simple_string, RespValue::SimpleString("PONG".to_string()));
    }

    #[test]
    fn parse_array() {
        let array = RespParser::parse_single(b"*2\r\n+hello\r\n+world\r\n").unwrap();
        assert_eq!(
            array,
            RespValue::Array(Box::from([
                RespValue::SimpleString("hello".to_string()),
                RespValue::SimpleString("world".to_string())
            ]))
        );
    }

    #[test]
    fn parse_bulk_string() {
        let bulk_string = RespParser::parse_single(b"$4\r\nECHO\r\n").unwrap();
        assert_eq!(bulk_string, RespValue::BulkString(BString::from("ECHO")));
    }

    #[test]
    fn parse_array_of_bulk_string_and_array_of_simple_string() {
        let array =
            RespParser::parse_single(b"*2\r\n$4\r\nquux\r\n*2\r\n+FOO\r\n$3\r\nbar\r\n").unwrap();
        assert_eq!(
            array,
            RespValue::Array(Box::from([
                RespValue::BulkString(BString::from("quux")),
                RespValue::Array(Box::from([
                    RespValue::SimpleString("FOO".to_string()),
                    RespValue::BulkString(BString::from("bar"))
                ]))
            ]))
        );
    }
}
