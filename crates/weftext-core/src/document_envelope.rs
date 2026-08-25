pub use weftext_asciidoc::{
    EnvelopeProbe as DocumentEnvelope, EnvelopeProbeState as DocumentEnvelopeState,
};

/// Probes only the leading YAML envelope delimiters and exact ranges.
///
/// Delimiter semantics are owned by `weftext-asciidoc`; Core only retains this
/// compatibility-shaped entry point for its callers.
#[must_use]
pub fn probe_document_envelope(source: &str) -> DocumentEnvelope {
    weftext_asciidoc::probe_managed_envelope(source)
}
