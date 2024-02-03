use anyhow::{bail, Context, Result};

const CRLF: &str = "\r\n";

const EMPTY_INPUT_ERROR_MSG: &str = "input cannot be empty";

#[derive(Debug, PartialEq, Eq)]
pub enum DataType {
    SimpleString(String),
    Array(Vec<DataType>),
}

impl DataType {
    fn parse_as_simple_string(value: &'_ str) -> Result<Self> {
        if value.chars().next().context(EMPTY_INPUT_ERROR_MSG)? != '+' {
            bail!("expected first character to be `+` for simple string");
        }
        let value = value[1..]
            .strip_suffix(CRLF)
            .context(format!(r"RESP lines should end in `{:?}`", CRLF))?;
        if value.contains('\n') || value.contains('\r') {
            bail!(r"input cannot contain `\r` or `\n`");
        }
        Ok(DataType::SimpleString(value.to_string()))
    }

    fn parse_as_array(value: &'_ str) -> Result<Self> {
        let mut lines = value.split_inclusive(CRLF);
        let first_line = lines.next().context("input cannot be empty")?;

        if first_line.chars().next().context(EMPTY_INPUT_ERROR_MSG)? != '*' {
            bail!("expected first character to be `*` for array");
        }
        let len = first_line[1..]
            .strip_suffix(CRLF)
            .expect("suffix not found (unreachable)")
            .parse::<u32>()
            .context("array length should be 32 bit decimal integer")?;

        let mut vec = Vec::with_capacity(len as usize);
        for line in lines {
            vec.push(Self::try_from(line)?);
        }
        if vec.len() != len as usize {
            bail!("expected `{}` array elements, got `{}`", len, vec.len());
        }

        Ok(DataType::Array(vec))
    }
}

impl TryFrom<&'_ str> for DataType {
    type Error = anyhow::Error;

    fn try_from(value: &'_ str) -> Result<Self> {
        // first ascii charactor determines the data type
        let Some(data_type_char) = value.chars().next() else {
            bail!("input cannot be empty");
        };

        match data_type_char {
            '+' => Self::parse_as_simple_string(value),
            '*' => Self::parse_as_array(value),
            _ => bail!("unsupported data type: `{}`", data_type_char),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DataType;

    #[test]
    fn parse_simple_string() {
        let simple_string = DataType::try_from("+PONG\r\n").unwrap();
        assert_eq!(simple_string, DataType::SimpleString("PONG".to_string()));
    }

    #[test]
    fn parse_array() {
        let array = DataType::try_from("*2\r\n+hello\r\n+world\r\n").unwrap();
        assert_eq!(
            array,
            DataType::Array(Vec::from([
                DataType::SimpleString("hello".to_string()),
                DataType::SimpleString("world".to_string())
            ]))
        )
    }
}
