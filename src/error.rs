use thiserror::Error;

/// A specialized [`Result`][] type to avoid writing out
/// [`dnstrie::Error`][enum@Error].
///
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for `dnstrie`
///
#[derive(Error, Debug)]
pub enum Error {
    #[error("name compression not allowed here")]
    Compression,
    #[error("arithmetic overflow")]
    FromInt(#[from] std::num::TryFromIntError),
    #[error("unsupported label type {0} (see RFC 6891)")]
    LabelType(u8),
    #[error("malformed DNS name: {0}")]
    NameFormat(&'static str),
}
