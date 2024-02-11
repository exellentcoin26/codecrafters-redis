#[derive(Debug)]
pub enum Error {
    EmptyInput,
    InvalidUtf8,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::EmptyInput => "cannot parse empty input",
                Self::InvalidUtf8 => "value is invalid utf-8",
            }
        )
    }
}
