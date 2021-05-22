use thiserror::Error;

/// A specialized [`Result`][] type to avoid writing out
/// [`dnstrie::Error`][enum@Error].
///
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for `dnstrie`
///
#[derive(Error, Debug)]
pub enum Error {
    #[error("DNS name has a bad compression pointer")]
    CompressBad,
    #[error("DNS name has chained compression pointers")]
    CompressChain,
    #[error("Character code {0} is too large")]
    EscapeBad(u16),
    #[error("Label length is inconcievable")]
    LabelLengthWat,
    #[error("Unsupported label type {0:#X}")]
    LabelType(u8),
    #[error("DNS name is too long")]
    NameLength,
    #[error("DNS name length is inconcievable")]
    NameLengthWat,
    #[error("Syntax error in domain name")]
    NameSyntax,
    #[error("DNS name has trailing junk")]
    NameTrailing,
    #[error("DNS name is truncated")]
    NameTruncated,
    #[error("DNS name is too long for its buffer")]
    ScratchOverflow,
    #[error("DNS name does not fit in WireLabels<u8>")]
    WideWire,
}
