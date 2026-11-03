//! The evidence pack adapter stub (RF-8).
//!
//! Gate evidence enters retroforge proposals only
//! as citations behind this adapter. The Gate Truth
//! evidence pack schema is not pinned for this line,
//! so the adapter consumes nothing: it refuses by
//! name, and no proposal carries gate citations.
//! Until the schema is pinned, any code path that
//! reaches for gate evidence fails loudly here
//! instead of guessing a format.

/// Gate evidence was requested before its format
/// was pinned. Carries the Python exception's
/// message.
#[derive(Debug)]
pub struct EvidencePackError(pub String);

/// Refuse: the evidence pack format is not pinned
/// yet.
pub fn gate_citations(_path: &str) -> Result<Vec<crate::util::Msg>, EvidencePackError> {
    Err(EvidencePackError(
        "the evidence pack format is not pinned yet, \
         no gate evidence can be cited"
            .to_string(),
    ))
}
