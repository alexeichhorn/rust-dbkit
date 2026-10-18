use half::f16;

/// A `halfvec(N)` with finite, half-precision elements.
#[derive(Debug, Clone, PartialEq)]
pub struct PgHalfVector<const N: usize> {
    values: [f16; N],
}

impl<const N: usize> PgHalfVector<N> {
    pub fn new(values: [f16; N]) -> Result<Self, PgHalfVectorError> {
        if !(1..=16_000).contains(&N) {
            return Err(PgHalfVectorError::InvalidDimension(N));
        }
        for (index, value) in values.iter().enumerate() {
            if !value.is_finite() {
                return Err(PgHalfVectorError::NonFinite { index });
            }
        }
        Ok(Self { values })
    }

    /// Rounds to half precision; values that overflow to infinity are rejected.
    pub fn from_f32(values: [f32; N]) -> Result<Self, PgHalfVectorError> {
        Self::new(values.map(f16::from_f32))
    }

    pub fn as_array(&self) -> &[f16; N] {
        &self.values
    }

    pub fn as_slice(&self) -> &[f16] {
        &self.values
    }

    #[cfg(feature = "sqlx")]
    fn decode_binary(bytes: &[u8]) -> Result<Self, PgHalfVectorError> {
        if bytes.len() < 4 || bytes[2..4] != [0, 0] {
            return Err(PgHalfVectorError::InvalidBinaryFormat);
        }
        let dimensions = usize::from(u16::from_be_bytes([bytes[0], bytes[1]]));
        if dimensions != N {
            return Err(PgHalfVectorError::DimensionMismatch {
                expected: N,
                actual: dimensions,
            });
        }
        if bytes.len() != 4 + dimensions * 2 {
            return Err(PgHalfVectorError::InvalidBinaryFormat);
        }
        Self::new(std::array::from_fn(|index| {
            let offset = 4 + index * 2;
            f16::from_bits(u16::from_be_bytes([bytes[offset], bytes[offset + 1]]))
        }))
    }
}

impl<const N: usize> std::str::FromStr for PgHalfVector<N> {
    type Err = PgHalfVectorError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let values = super::PgVector::<N>::parse_text(input).map_err(|_| PgHalfVectorError::InvalidTextFormat(input.into()))?;
        let values = values.try_into().map_err(|values: Vec<f32>| PgHalfVectorError::DimensionMismatch {
            expected: N,
            actual: values.len(),
        })?;
        Self::from_f32(values)
    }
}

impl<const N: usize> std::fmt::Display for PgHalfVector<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[")?;
        for (index, value) in self.values.iter().enumerate() {
            if index > 0 {
                f.write_str(",")?;
            }
            write!(f, "{value}")?;
        }
        f.write_str("]")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PgHalfVectorError {
    #[error("halfvec dimensions must be between 1 and 16000, got {0}")]
    InvalidDimension(usize),
    #[error("halfvec dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    #[error("halfvec contains a non-finite value at index {index}")]
    NonFinite { index: usize },
    #[error("invalid halfvec text format: {0}")]
    InvalidTextFormat(String),
    #[error("invalid halfvec binary format")]
    InvalidBinaryFormat,
}

#[cfg(feature = "sqlx")]
impl<const N: usize> sqlx::Type<sqlx::Postgres> for PgHalfVector<N> {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("halfvec")
    }
}

#[cfg(feature = "sqlx")]
impl<'r, const N: usize> sqlx::Decode<'r, sqlx::Postgres> for PgHalfVector<N> {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        Ok(match value.format() {
            sqlx::postgres::PgValueFormat::Text => value.as_str()?.parse()?,
            sqlx::postgres::PgValueFormat::Binary => Self::decode_binary(value.as_bytes()?)?,
        })
    }
}

#[cfg(all(test, feature = "sqlx"))]
mod tests {
    use super::PgHalfVector;

    #[test]
    fn binary_decoder_rejects_truncation_reserved_fields_and_non_finite_values() {
        let bytes = [0, 2, 0, 0, 0x3c, 0, 0x38, 0];
        assert_eq!(
            PgHalfVector::<2>::decode_binary(&bytes).unwrap(),
            PgHalfVector::<2>::from_f32([1.0, 0.5]).unwrap()
        );
        for end in 0..bytes.len() {
            assert!(PgHalfVector::<2>::decode_binary(&bytes[..end]).is_err());
        }
        let mut reserved = bytes;
        reserved[3] = 1;
        assert!(PgHalfVector::<2>::decode_binary(&reserved).is_err());
        let mut infinity = bytes;
        infinity[4] = 0x7c;
        assert!(PgHalfVector::<2>::decode_binary(&infinity).is_err());
    }
}
