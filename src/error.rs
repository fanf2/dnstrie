use thiserror::Error;

/// A specialized [`Result`][] type to avoid writing out
/// [`dnstrie::Error`][enum@Error].
///
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for `dnstrie`
///
#[derive(Debug, Error)]
pub enum Error {
    #[error("Conversion is inconcievable")]
    BugFromInt(#[from] std::num::TryFromIntError),
    #[error("Wire position is inconcievable ({0})")]
    BugWirePos(usize),
    #[error("Scratch pad is inconcievable ({0})")]
    BugScratchPad(&'static str),
    #[error("DNS name has a bad compression pointer")]
    CompressBad,
    #[error("DNS name has chained compression pointers")]
    CompressChain,
    #[error("Bad character code: {0}")]
    EscapeBad(#[from] std::num::ParseIntError),
    #[error("Unsupported label type {0:#X}")]
    LabelType(u8),
    #[error("DNS name is too long")]
    NameLength,
    #[error("Syntax error in domain name")]
    NameSyntax,
    #[error("DNS name has trailing junk")]
    NameTrailing,
    #[error("DNS name is truncated")]
    NameTruncated,
    #[error("DNS name is too long for its buffer")]
    ScratchOverflow,
    #[error("Bad UTF-8: {0}")]
    Utf8Bad(#[from] std::str::Utf8Error),
    #[error("DNS name does not fit in WireLabels<u8>")]
    WideWire,
}
