use thiserror::Error;

/// A specialized [`Result`][] type to avoid writing out
/// [`dnstrie::Error`][enum@Error].
///
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for `dnstrie`
///
#[derive(Error, Debug)]
pub enum Error {
    #[error("DNS name compression not allowed")]
    CompressBan,
    #[error("DNS name has chained compression pointers")]
    CompressChain,
    #[error("DNS name has wild compression pointer")]
    CompressWild,
    #[error("arithmetic overflow")]
    FromInt(#[from] std::num::TryFromIntError),
    #[error("Domain name label is too long")]
    LabelLength,
    #[error("unsupported label type {0} (see RFC 6891)")]
    LabelType(u8),
    #[error("DNS name has too many labels")]
    NameLabels,
    #[error("DNS name is too long")]
    NameLength,
    #[error("Domain name contains \"")]
    NameQuotes,
    #[error("DNS name is truncated")]
    NameTruncated,
}
