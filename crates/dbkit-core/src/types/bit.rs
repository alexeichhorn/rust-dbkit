/// A fixed-length `bit(N)` value packed most-significant bit first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgBit<const N: usize> {
    bytes: Vec<u8>,
}

impl<const N: usize> PgBit<N> {
    pub fn new(values: [bool; N]) -> Result<Self, PgBitError> {
        Self::from_slice(&values)
    }

    fn from_slice(values: &[bool]) -> Result<Self, PgBitError> {
        Self::validate_dimensions()?;
        if values.len() != N {
            return Err(PgBitError::DimensionMismatch {
                expected: N,
                actual: values.len(),
            });
        }
        let mut bytes = vec![0; N.div_ceil(8)];
        for (index, &value) in values.iter().enumerate() {
            if value {
                bytes[index / 8] |= 1 << (7 - index % 8);
            }
        }
        Ok(Self { bytes })
    }

    /// Unused low bits in the last byte are zero.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn validate_dimensions() -> Result<(), PgBitError> {
        if N == 0 || N > (i32::MAX as usize - 7) {
            return Err(PgBitError::InvalidDimension(N));
        }
        Ok(())
    }

    #[cfg(feature = "sqlx")]
    fn decode_binary(bytes: &[u8]) -> Result<Self, PgBitError> {
        Self::validate_dimensions()?;
        if bytes.len() < 4 {
            return Err(PgBitError::InvalidBinaryFormat);
        }
        let dimensions = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if dimensions != N {
            return Err(PgBitError::DimensionMismatch {
                expected: N,
                actual: dimensions,
            });
        }
        if bytes.len() != 4 + N.div_ceil(8) {
            return Err(PgBitError::InvalidBinaryFormat);
        }
        if !N.is_multiple_of(8) && bytes[bytes.len() - 1] & (0xff >> (N % 8)) != 0 {
            return Err(PgBitError::InvalidBinaryFormat);
        }
        Ok(Self {
            bytes: bytes[4..].to_vec(),
        })
    }
}

impl<const N: usize> TryFrom<Vec<bool>> for PgBit<N> {
    type Error = PgBitError;

    fn try_from(values: Vec<bool>) -> Result<Self, Self::Error> {
        Self::from_slice(&values)
    }
}

impl<const N: usize> std::str::FromStr for PgBit<N> {
    type Err = PgBitError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::validate_dimensions()?;
        if input.len() != N {
            return Err(PgBitError::DimensionMismatch {
                expected: N,
                actual: input.len(),
            });
        }
        let mut bytes = vec![0; N.div_ceil(8)];
        for (index, value) in input.bytes().enumerate() {
            match value {
                b'0' => {}
                b'1' => bytes[index / 8] |= 1 << (7 - index % 8),
                _ => return Err(PgBitError::InvalidTextFormat(input.into())),
            }
        }
        Ok(Self { bytes })
    }
}

impl<const N: usize> std::fmt::Display for PgBit<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for index in 0..N {
            f.write_str(if self.bytes[index / 8] & (1 << (7 - index % 8)) == 0 {
                "0"
            } else {
                "1"
            })?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PgBitError {
    #[error("bit dimensions must be between 1 and 2147483640, got {0}")]
    InvalidDimension(usize),
    #[error("bit dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    #[error("invalid bit text format: {0}")]
    InvalidTextFormat(String),
    #[error("invalid bit binary format")]
    InvalidBinaryFormat,
}

#[cfg(feature = "sqlx")]
impl<const N: usize> sqlx::Type<sqlx::Postgres> for PgBit<N> {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("bit")
    }
}

#[cfg(feature = "sqlx")]
impl<'r, const N: usize> sqlx::Decode<'r, sqlx::Postgres> for PgBit<N> {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        Ok(match value.format() {
            sqlx::postgres::PgValueFormat::Text => value.as_str()?.parse()?,
            sqlx::postgres::PgValueFormat::Binary => Self::decode_binary(value.as_bytes()?)?,
        })
    }
}

#[cfg(all(test, feature = "sqlx"))]
mod tests {
    use super::PgBit;

    #[test]
    fn binary_decoder_rejects_truncation_trailing_bytes_and_nonzero_padding() {
        let bytes = [0, 0, 0, 9, 0b0100_0000, 0b1000_0000];
        assert_eq!(PgBit::<9>::decode_binary(&bytes).unwrap().to_string(), "010000001");
        for end in 0..bytes.len() {
            assert!(PgBit::<9>::decode_binary(&bytes[..end]).is_err());
        }
        let mut trailing = bytes.to_vec();
        trailing.push(0);
        assert!(PgBit::<9>::decode_binary(&trailing).is_err());
        let mut padding = bytes;
        padding[5] |= 1;
        assert!(PgBit::<9>::decode_binary(&padding).is_err());
    }
}
