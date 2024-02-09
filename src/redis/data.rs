use anyhow::{bail, Context, Result};

const CRLF: &str = "\r\n";

const EMPTY_INPUT_ERROR_MSG: &str = "cannot parse empty resp string";

#[derive(Debug, PartialEq, Eq)]
pub enum DataType {
    SimpleString(String),
    BulkString(Box<[u8]>),
    Array(Box<[DataType]>),
}

impl DataType {
    pub fn from_resp(value: &'_ str) -> Result<Self> {
        dbg!(value);
        let mut lines = value.split(CRLF);
        let result = Self::from_resp_iter(&mut lines)?;
        let mut remainder = lines.peekable();
        Ok(match remainder.peek() {
            Some(&"") | None => result,
            Some(..) => {
                unimplemented!(
                    "could not parse resp string as single value (remaining: {:?})",
                    remainder.fold(String::new(), |remainder, l| remainder + l)
                )
            }
        })
    }

    fn from_resp_iter<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Result<Self> {
        let mut first_line = lines.next().context(EMPTY_INPUT_ERROR_MSG)?.chars();
        let Some(first_char) = first_line.next() else {
            bail!("resp line must have a first character indicating the datatype");
        };

        Ok(match first_char {
            '+' => Self::parse_as_simple_string(first_line.as_str())?,
            '*' => Self::parse_as_array(Self::parse_as_u32(first_line.as_str())?, lines)?,
            '$' => Self::parse_as_bulk_string(
                Self::parse_as_u32(first_line.as_str())?,
                lines
                    .next()
                    .context("missing second line for bulk string data")?,
            )?,
            _ => bail!("unimplemented datatype `{}`", first_char),
        })
    }

    fn parse_as_u32(value: &'_ str) -> Result<u32> {
        value
            .parse::<u32>()
            .context("failed to parse value as 32 bit unsigned integer")
    }

    fn parse_as_simple_string(value: &'_ str) -> Result<Self> {
        if value.contains('\n') || value.contains('\r') {
            bail!(r"input cannot contain `\r` or `\n`");
        }
        Ok(DataType::SimpleString(value.to_string()))
    }

    fn parse_as_array<'a>(length: u32, lines: &mut impl Iterator<Item = &'a str>) -> Result<Self> {
        Ok(DataType::Array(
            (0..length)
                .into_iter()
                .map(|_| Self::from_resp_iter(&mut *lines))
                .collect::<Result<_>>()?,
        ))
    }

    fn parse_as_bulk_string(length: u32, value: &'_ str) -> Result<Self> {
        if value.len() != length as usize {
            bail!("bulk string value size is not equal to the expected size (expected: `{}`, got: `{}`)", value.len(), length);
        }

        Ok(DataType::BulkString(Box::from(value.as_bytes())))
    }
}

#[cfg(test)]
mod tests {
    use super::DataType;

    #[test]
    fn parse_simple_string() {
        let simple_string = DataType::from_resp("+PONG\r\n").unwrap();
        assert_eq!(simple_string, DataType::SimpleString("PONG".to_string()));
    }

    #[test]
    fn parse_array() {
        let array = DataType::from_resp("*2\r\n+hello\r\n+world\r\n").unwrap();
        assert_eq!(
            array,
            DataType::Array(Box::from([
                DataType::SimpleString("hello".to_string()),
                DataType::SimpleString("world".to_string())
            ]))
        );
    }

    #[test]
    fn parse_bulk_string() {
        let bulk_string = DataType::from_resp("$4\r\nECHO\r\n").unwrap();
        assert_eq!(
            bulk_string,
            DataType::BulkString(Box::from("ECHO".as_bytes()))
        );
    }

    #[test]
    fn parse_array_of_bulk_string_and_array_of_simple_string() {
        let array = DataType::from_resp("*2\r\n$4\r\nquux\r\n*2\r\n+FOO\r\n$3\r\nbar\r\n").unwrap();
        assert_eq!(
            array,
            DataType::Array(Box::from([
                DataType::BulkString(Box::from("quux".as_bytes())),
                DataType::Array(Box::from([
                    DataType::SimpleString("FOO".to_string()),
                    DataType::BulkString(Box::from("bar".as_bytes()))
                ]))
            ]))
        );
    }
}
