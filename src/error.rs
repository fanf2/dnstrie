use thiserror::Error;

/// A specialized [`Result`][] type to avoid writing out
/// [`dnstrie::Error`][enum@Error].
///
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for `dnstrie`
///
#[derive(Error, Debug)]
pub enum Error {
    #[error("arithmetic overflow")]
    FromInt(#[from] std::num::TryFromIntError),
    #[error("unsupported label type {0} (see RFC 6891)")]
    LabelType(u8),
    #[error("DNS name has too many labels")]
    NameLabels,
    #[error("DNS name is too long")]
    NameLength,
    #[error("DNS name is truncated")]
    NameTruncated,
    #[error("DNS name compression not allowed")]
    CompressBan,
    #[error("DNS name has chained compression pointers")]
    CompressChain,
    #[error("DNS name has wild compression pointer")]
    CompressWild,
}
