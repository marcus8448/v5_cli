#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Generic(&'static str),
    InvalidId(u8),
    InvalidName(String),
    Unknown
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::IO(error)
    }
}
