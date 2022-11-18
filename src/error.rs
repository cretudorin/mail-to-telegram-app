#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("An IO Error occured: {:?}", .0)]
    IOError(#[from] std::io::Error),
    #[error("Can't parse host address")]
    SocketAddrParseError,
    #[error("Logger could not be set")]
    LoggerSetError(#[from] log::SetLoggerError)
}