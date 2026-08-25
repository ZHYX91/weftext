use serde::{Deserialize, Serialize};

use crate::{
    ImportAdapter, ImportError, ImportErrorCode, ImportLimits, Sha256Digest, SourceArtifact,
    sha256_bytes,
};

pub const PROBE_EVIDENCE_CONTRACT_VERSION: &str = "weftext.import.probe-evidence.v1";

/// One byte-exact, offset-bound range inspected by a format probe.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProbeEvidenceSegment {
    pub offset: u64,
    pub byte_length: u64,
    pub sha256: Sha256Digest,
}

/// Closed evidence inventory for a bounded, random-access format probe.
///
/// Segment ranges are sorted, non-overlapping, and non-adjacent. Their union,
/// rather than the number of read calls, is charged to the probe byte budget.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProbeEvidence {
    pub contract_version: String,
    pub source_digest: Sha256Digest,
    pub source_byte_length: u64,
    pub byte_budget: u64,
    pub inspected_bytes: u64,
    pub segments: Vec<ProbeEvidenceSegment>,
}

impl ProbeEvidence {
    pub(crate) fn validate(
        &self,
        source: &SourceArtifact,
        limits: &ImportLimits,
    ) -> Result<(), ImportError> {
        if self.contract_version != PROBE_EVIDENCE_CONTRACT_VERSION
            || self.source_digest != source.sha256
            || self.source_byte_length != source.byte_length
            || self.byte_budget != limits.max_probe_bytes
            || self.inspected_bytes > self.byte_budget
            || self.segments.len() > 4_096
        {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "format probe evidence differs from its source or bounded limit",
            ));
        }
        let mut previous_end = 0_u64;
        let mut total = 0_u64;
        for (index, segment) in self.segments.iter().enumerate() {
            if segment.byte_length == 0 {
                return Err(ImportError::new(
                    ImportErrorCode::InvalidContract,
                    "format probe evidence cannot contain an empty segment",
                ));
            }
            let end = segment
                .offset
                .checked_add(segment.byte_length)
                .ok_or_else(|| {
                    ImportError::new(
                        ImportErrorCode::InvalidContract,
                        "format probe evidence range overflowed",
                    )
                })?;
            if end > source.byte_length || (index > 0 && segment.offset <= previous_end) {
                return Err(ImportError::new(
                    ImportErrorCode::InvalidContract,
                    "format probe evidence ranges overlap, touch, or escape the source",
                ));
            }
            previous_end = end;
            total = total.checked_add(segment.byte_length).ok_or_else(|| {
                ImportError::new(
                    ImportErrorCode::InvalidContract,
                    "format probe evidence byte total overflowed",
                )
            })?;
        }
        if total != self.inspected_bytes {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "format probe evidence byte total is inconsistent",
            ));
        }
        Ok(())
    }
}

/// Adapter-facing bounded random-access reader over exact source bytes.
///
/// The reader never exposes the complete source implicitly. Every returned
/// byte range is retained as offset/digest evidence and the union of all ranges
/// is limited by `ImportLimits::max_probe_bytes`.
pub struct ProbeReader<'a> {
    bytes: &'a [u8],
    source_digest: Sha256Digest,
    byte_budget: u64,
    ranges: Vec<(usize, usize)>,
}

impl<'a> ProbeReader<'a> {
    fn new(
        source: &SourceArtifact,
        bytes: &'a [u8],
        limits: &ImportLimits,
    ) -> Result<Self, ImportError> {
        limits.validate()?;
        let byte_length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if source.byte_length != byte_length || source.sha256 != sha256_bytes(bytes) {
            return Err(ImportError::new(
                ImportErrorCode::InvalidSource,
                "format probe bytes differ from the exact source artifact",
            ));
        }
        Ok(Self {
            bytes,
            source_digest: source.sha256.clone(),
            byte_budget: limits.max_probe_bytes,
            ranges: Vec::new(),
        })
    }

    #[must_use]
    pub fn source_byte_length(&self) -> u64 {
        u64::try_from(self.bytes.len()).unwrap_or(u64::MAX)
    }

    #[must_use]
    pub fn remaining_budget(&self) -> u64 {
        self.byte_budget.saturating_sub(self.inspected_bytes())
    }

    /// Reads an exact source range and charges only newly inspected bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested range escapes the source, overflows,
    /// or would exceed the total probe budget.
    pub fn read_at(&mut self, offset: u64, byte_length: u64) -> Result<Vec<u8>, ImportError> {
        if byte_length == 0 {
            return Ok(Vec::new());
        }
        let start = usize::try_from(offset).map_err(|_| range_error())?;
        let length = usize::try_from(byte_length).map_err(|_| range_error())?;
        let end = start.checked_add(length).ok_or_else(range_error)?;
        if end > self.bytes.len() {
            return Err(range_error());
        }
        let mut candidate = self.ranges.clone();
        candidate.push((start, end));
        normalize_ranges(&mut candidate);
        let inspected = inspected_bytes(&candidate);
        if inspected > self.byte_budget {
            return Err(ImportError::new(
                ImportErrorCode::LimitExceeded,
                "format probe exhausted its total random-access byte budget",
            ));
        }
        self.ranges = candidate;
        Ok(self.bytes[start..end].to_vec())
    }

    /// Reads up to `maximum_bytes` beginning at an exact offset.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid offset or exhausted evidence budget.
    pub fn read_up_to(&mut self, offset: u64, maximum_bytes: u64) -> Result<Vec<u8>, ImportError> {
        if offset > self.source_byte_length() {
            return Err(range_error());
        }
        let available = self.source_byte_length().saturating_sub(offset);
        self.read_at(offset, available.min(maximum_bytes))
    }

    /// Reads a bounded leading range.
    ///
    /// # Errors
    ///
    /// Returns an error when the evidence budget is exhausted.
    pub fn read_head(&mut self, maximum_bytes: u64) -> Result<Vec<u8>, ImportError> {
        self.read_up_to(0, maximum_bytes)
    }

    /// Reads a bounded trailing range.
    ///
    /// # Errors
    ///
    /// Returns an error when the evidence budget is exhausted.
    pub fn read_tail(&mut self, maximum_bytes: u64) -> Result<Vec<u8>, ImportError> {
        let length = self.source_byte_length().min(maximum_bytes);
        self.read_at(self.source_byte_length().saturating_sub(length), length)
    }

    #[must_use]
    pub fn evidence(&self) -> ProbeEvidence {
        let segments = self
            .ranges
            .iter()
            .map(|(start, end)| ProbeEvidenceSegment {
                offset: u64::try_from(*start).unwrap_or(u64::MAX),
                byte_length: u64::try_from(end.saturating_sub(*start)).unwrap_or(u64::MAX),
                sha256: sha256_bytes(&self.bytes[*start..*end]),
            })
            .collect();
        ProbeEvidence {
            contract_version: PROBE_EVIDENCE_CONTRACT_VERSION.to_owned(),
            source_digest: self.source_digest.clone(),
            source_byte_length: self.source_byte_length(),
            byte_budget: self.byte_budget,
            inspected_bytes: self.inspected_bytes(),
            segments,
        }
    }

    fn inspected_bytes(&self) -> u64 {
        inspected_bytes(&self.ranges)
    }
}

/// Replays an adapter probe from exact source bytes through the shared bounded
/// random-access evidence authority.
///
/// The common boundary seeds both ends of every non-empty source, then permits
/// the selected adapter to request additional exact offsets. The returned
/// probe must contain the exact evidence inventory produced by those reads.
///
/// # Errors
///
/// Returns an error for inconsistent source bytes, an evidence-budget breach,
/// or an adapter result that does not preserve the common evidence authority.
pub fn probe_source_bytes(
    adapter: &dyn ImportAdapter,
    source: &SourceArtifact,
    bytes: &[u8],
    limits: &ImportLimits,
) -> Result<crate::FormatProbe, ImportError> {
    probe_source_bytes_with(source, bytes, limits, |reader| {
        adapter.probe(source, reader, limits)
    })
}

pub(crate) fn probe_source_bytes_with(
    source: &SourceArtifact,
    bytes: &[u8],
    limits: &ImportLimits,
    derive: impl FnOnce(&mut ProbeReader<'_>) -> Result<crate::FormatProbe, ImportError>,
) -> Result<crate::FormatProbe, ImportError> {
    let mut reader = ProbeReader::new(source, bytes, limits)?;
    if !bytes.is_empty() {
        let edge = limits.max_probe_bytes.saturating_div(2).clamp(1, 16);
        reader.read_head(edge)?;
        reader.read_tail(edge)?;
    }
    let probe = derive(&mut reader)?;
    let evidence = reader.evidence();
    if probe.evidence != evidence {
        return Err(ImportError::new(
            ImportErrorCode::InvalidContract,
            "format adapter did not preserve the common probe evidence inventory",
        ));
    }
    probe.validate(source, limits)?;
    Ok(probe)
}

fn normalize_ranges(ranges: &mut Vec<(usize, usize)>) {
    ranges.sort_unstable_by_key(|range| range.0);
    let mut normalized: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
    for &(start, end) in ranges.iter() {
        if let Some(last) = normalized.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(end);
            continue;
        }
        normalized.push((start, end));
    }
    *ranges = normalized;
}

fn inspected_bytes(ranges: &[(usize, usize)]) -> u64 {
    ranges.iter().fold(0_u64, |total, (start, end)| {
        total.saturating_add(u64::try_from(end.saturating_sub(*start)).unwrap_or(u64::MAX))
    })
}

fn range_error() -> ImportError {
    ImportError::new(
        ImportErrorCode::InvalidSource,
        "format probe requested a range outside the exact source bytes",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OriginClass, SourceArtifact};

    #[test]
    fn random_access_evidence_is_offset_bound_deduplicated_and_budgeted() {
        let limits = ImportLimits {
            max_source_bytes: 64,
            max_probe_bytes: 8,
            ..ImportLimits::default()
        };
        let bytes = b"0123456789abcdef";
        let source =
            SourceArtifact::from_bytes("fixture.bin", OriginClass::TestFixture, bytes, &limits)
                .expect("source");
        let mut reader = ProbeReader::new(&source, bytes, &limits).expect("reader");
        assert_eq!(reader.read_at(0, 4).unwrap(), b"0123");
        assert_eq!(reader.read_at(2, 4).unwrap(), b"2345");
        assert_eq!(reader.read_at(14, 2).unwrap(), b"ef");
        assert_eq!(reader.remaining_budget(), 0);
        assert_eq!(
            reader.read_at(8, 1).expect_err("budget").code(),
            ImportErrorCode::LimitExceeded
        );
        let evidence = reader.evidence();
        assert_eq!(evidence.inspected_bytes, 8);
        assert_eq!(evidence.segments.len(), 2);
        evidence.validate(&source, &limits).expect("evidence");
    }
}
