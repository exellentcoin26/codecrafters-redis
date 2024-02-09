use crate::redis::DataType;
use anyhow::{bail, Context, Result};

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Ping,
    Echo { value: String },
}

impl Command {
    pub fn parse_from_wire(value: &'_ str) -> Result<Self> {
        let data = DataType::from_resp(value)?;

        match data {
            DataType::SimpleString(command) => Self::parse_from_simple_string(&command),
            DataType::Array(array) => Self::parse_from_array(array),
            _ => bail!("unsupported data type for command"),
        }
    }

    fn parse_from_simple_string(command: &'_ str) -> Result<Self> {
        Ok(match command.to_lowercase().as_str() {
            "ping" => Command::Ping,
            _ => bail!("unexpected command `{}`", command),
        })
    }

    fn parse_from_array(array: Box<[DataType]>) -> Result<Self> {
        let mut array = array.into_vec().into_iter();
        let command = match array
            .next()
            .context("input array for command cannot be empty")?
        {
            DataType::SimpleString(command) => command,
            DataType::BulkString(command) => String::from_utf8(command.into_vec())
                .context("command name can only contain utf-8 characters")?,
            _ => bail!("unexpeced data type as command name"),
        };

        match command.to_lowercase().as_str() {
            "echo" => Self::parse_as_echo_args(array),
            _ => bail!("unexpected command `{}`", command),
        }
    }

    fn parse_as_echo_args(mut args: impl Iterator<Item = DataType>) -> Result<Self> {
        let value = match args
            .next()
            .context("echo command expects a single argument")?
        {
            DataType::SimpleString(value) => value,
            DataType::BulkString(command) => String::from_utf8(command.into_vec())
                .context("command name can only contain utf-8 characters")?,
            _ => bail!("unexpected data type as echo command argument"),
        };
        Ok(Command::Echo { value })
    }
}

#[cfg(test)]
mod tests {
    use super::Command;

    #[test]
    fn parse_pong_command() {
        let command = Command::parse_from_wire("+PING\r\n").unwrap();
        assert_eq!(command, Command::Ping);
    }

    #[test]
    fn parse_echo_simple_string() {
        let command = Command::parse_from_wire("*2\r\n+ECHO\r\n+Hello, World!\r\n").unwrap();
        assert_eq!(
            command,
            Command::Echo {
                value: "Hello, World!".to_string()
            }
        )
    }
}
