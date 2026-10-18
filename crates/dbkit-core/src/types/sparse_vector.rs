/// A `sparsevec(N)` storing sorted, nonzero entries with zero-based Rust indices.
#[derive(Debug, Clone, PartialEq)]
pub struct PgSparseVector<const N: usize> {
    entries: Vec<(usize, f32)>,
}

impl<const N: usize> PgSparseVector<N> {
    /// Sorts entries and drops zeros, rejecting duplicates before dropping zeros.
    pub fn new(entries: impl IntoIterator<Item = (usize, f32)>) -> Result<Self, PgSparseVectorError> {
        if !(1..=1_000_000_000).contains(&N) {
            return Err(PgSparseVectorError::InvalidDimension(N));
        }
        let mut entries: Vec<_> = entries.into_iter().collect();
        entries.sort_unstable_by_key(|(index, _)| *index);
        let mut previous = None;
        for &(index, value) in &entries {
            if index >= N {
                return Err(PgSparseVectorError::IndexOutOfBounds { index, dimensions: N });
            }
            if previous == Some(index) {
                return Err(PgSparseVectorError::DuplicateIndex { index });
            }
            if !value.is_finite() {
                return Err(PgSparseVectorError::NonFinite { index });
            }
            previous = Some(index);
        }
        entries.retain(|(_, value)| *value != 0.0);
        if entries.len() > 16_000 {
            return Err(PgSparseVectorError::TooManyEntries(entries.len()));
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[(usize, f32)] {
        &self.entries
    }

    #[cfg(feature = "sqlx")]
    fn decode_binary(bytes: &[u8]) -> Result<Self, PgSparseVectorError> {
        if bytes.len() < 12 || bytes[8..12] != [0; 4] {
            return Err(PgSparseVectorError::InvalidBinaryFormat);
        }
        let dimensions = u32::from_be_bytes(bytes[..4].try_into()?) as usize;
        let count = u32::from_be_bytes(bytes[4..8].try_into()?) as usize;
        if dimensions != N {
            return Err(PgSparseVectorError::DimensionMismatch {
                expected: N,
                actual: dimensions,
            });
        }
        if count > 16_000 || bytes.len() != 12 + count * 8 {
            return Err(PgSparseVectorError::InvalidBinaryFormat);
        }
        let (indices, values) = bytes[12..].split_at(count * 4);
        let mut entries = Vec::with_capacity(count);
        for (index, value) in indices.chunks_exact(4).zip(values.chunks_exact(4)) {
            entries.push((
                u32::from_be_bytes(index.try_into()?) as usize,
                f32::from_be_bytes(value.try_into()?),
            ));
        }
        Self::new(entries)
    }
}

impl<const N: usize> std::str::FromStr for PgSparseVector<N> {
    type Err = PgSparseVectorError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let invalid = || PgSparseVectorError::InvalidTextFormat(input.into());
        let (entries, dimensions) = input.trim().rsplit_once('/').ok_or_else(invalid)?;
        let dimensions = dimensions.trim().parse::<usize>().map_err(|_| invalid())?;
        if dimensions != N {
            return Err(PgSparseVectorError::DimensionMismatch {
                expected: N,
                actual: dimensions,
            });
        }
        let entries = entries
            .trim()
            .strip_prefix('{')
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(invalid)?;
        let mut parsed = Vec::new();
        if !entries.trim().is_empty() {
            for entry in entries.split(',') {
                let (index, value) = entry.split_once(':').ok_or_else(invalid)?;
                let index = index
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| invalid())?
                    .checked_sub(1)
                    .ok_or_else(invalid)?;
                let value = value.trim().parse::<f32>().map_err(|_| invalid())?;
                parsed.push((index, value));
            }
        }
        Self::new(parsed)
    }
}

impl<const N: usize> std::fmt::Display for PgSparseVector<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("{")?;
        for (position, &(index, value)) in self.entries.iter().enumerate() {
            if position > 0 {
                f.write_str(",")?;
            }
            let index = index + 1;
            write!(f, "{index}:{value}")?;
        }
        write!(f, "}}/{N}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PgSparseVectorError {
    #[error("sparsevec dimensions must be between 1 and 1000000000, got {0}")]
    InvalidDimension(usize),
    #[error("sparsevec dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    #[error("sparsevec index {index} is out of bounds for {dimensions} dimensions")]
    IndexOutOfBounds { index: usize, dimensions: usize },
    #[error("sparsevec repeats index {index}")]
    DuplicateIndex { index: usize },
    #[error("sparsevec contains a non-finite value at index {index}")]
    NonFinite { index: usize },
    #[error("sparsevec cannot have more than 16000 nonzero entries, got {0}")]
    TooManyEntries(usize),
    #[error("invalid sparsevec text format: {0}")]
    InvalidTextFormat(String),
    #[error("invalid sparsevec binary format")]
    InvalidBinaryFormat,
}

#[cfg(feature = "sqlx")]
impl From<std::array::TryFromSliceError> for PgSparseVectorError {
    fn from(_: std::array::TryFromSliceError) -> Self {
        Self::InvalidBinaryFormat
    }
}

#[cfg(feature = "sqlx")]
impl<const N: usize> sqlx::Type<sqlx::Postgres> for PgSparseVector<N> {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("sparsevec")
    }
}

#[cfg(feature = "sqlx")]
impl<'r, const N: usize> sqlx::Decode<'r, sqlx::Postgres> for PgSparseVector<N> {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        Ok(match value.format() {
            sqlx::postgres::PgValueFormat::Text => value.as_str()?.parse()?,
            sqlx::postgres::PgValueFormat::Binary => Self::decode_binary(value.as_bytes()?)?,
        })
    }
}

#[cfg(all(test, feature = "sqlx"))]
mod tests {
    use super::PgSparseVector;

    #[test]
    fn binary_decoder_rejects_truncation_invalid_counts_and_out_of_range_indices() {
        let bytes = [0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0x3f, 0x80, 0, 0];
        assert_eq!(PgSparseVector::<3>::decode_binary(&bytes).unwrap().entries(), &[(2, 1.0)]);
        for end in 0..bytes.len() {
            assert!(PgSparseVector::<3>::decode_binary(&bytes[..end]).is_err());
        }
        let mut count = bytes;
        count[4..8].fill(0xff);
        assert!(PgSparseVector::<3>::decode_binary(&count).is_err());
        let mut index = bytes;
        index[15] = 3;
        assert!(PgSparseVector::<3>::decode_binary(&index).is_err());
    }
}
