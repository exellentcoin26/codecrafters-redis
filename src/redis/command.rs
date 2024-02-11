use crate::redis::RespValue;
use anyhow::{bail, Context, Result};
use bstr::{BStr, BString, ByteSlice};

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Ping,
    Echo { value: BString },
}

impl Command {
    pub fn to_wire(&self) -> std::io::Result<BString> {
        let mut result = vec![];
        match self {
            Command::Ping => RespValue::SimpleString("PONG".to_string()),
            Command::Echo { value } => RespValue::BulkString(value.clone()),
        }
        .write_resp(&mut result)?;
        Ok(BString::from(result))
    }

    pub fn from_wire(wire: &'_ [u8]) -> Result<Self> {
        let value = RespValue::from_resp(wire)?;
        Self::parse_single_command(value)
    }

    fn parse_single_command(value: RespValue) -> Result<Self> {
        match value {
            RespValue::SimpleString(command) => Self::from_simple_string(&command),
            RespValue::BulkString(command) => Self::from_bulk_string(command.as_bstr()),
            RespValue::Array(arr) => Self::from_array(arr),
        }
    }

    fn from_bulk_string(command: &BStr) -> Result<Self> {
        let command = std::str::from_utf8(command)
            .context("command name contains invalid utf-8 characters")?;
        Self::from_simple_string(command)
    }

    fn from_simple_string(command: &'_ str) -> Result<Self> {
        log::debug!("Command: {}", command.to_lowercase().as_str());
        Ok(match command.to_lowercase().as_str() {
            "ping" => Command::Ping,
            _ => bail!("unexpected command `{}`", command),
        })
    }

    fn from_array(array: Box<[RespValue]>) -> Result<Self> {
        let mut array = array.into_vec().into_iter();
        let command = match array
            .next()
            .context("input array for command cannot be empty")?
        {
            RespValue::SimpleString(command) => command,
            RespValue::BulkString(command) => String::try_from(command)?,
            _ => bail!("unexpeced data type as command name"),
        };

        Ok(match command.to_lowercase().as_str() {
            "echo" => Self::take_as_echo_args(&mut array)?,
            "ping" => Self::Ping,
            _ => bail!("unexpected command `{}`", command),
        })
    }

    fn take_as_echo_args(args: &mut impl Iterator<Item = RespValue>) -> Result<Self> {
        let value = match args
            .next()
            .context("echo command expects a single argument")?
        {
            RespValue::SimpleString(value) => BString::from(value),
            RespValue::BulkString(command) => command,
            _ => bail!("unexpected data type as echo command argument"),
        };
        Ok(Command::Echo { value })
    }
}

#[cfg(test)]
mod tests {
    use super::Command;
    use bstr::BString;

    #[test]
    fn parse_ping_command() {
        let command = Command::from_wire(b"+PING\r\n").unwrap();
        assert_eq!(command, Command::Ping);
    }

    #[test]
    fn parse_echo_simple_string() {
        let command = Command::from_wire(b"*2\r\n+ECHO\r\n+Hello, World!\r\n").unwrap();
        assert_eq!(
            command,
            Command::Echo {
                value: BString::from("Hello, World!")
            }
        )
    }

    #[test]
    fn parse_echo_bulk_string() {
        let command = Command::from_wire(b"*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n").unwrap();
        assert_eq!(
            command,
            Command::Echo {
                value: BString::from("hey")
            }
        )
    }
}
