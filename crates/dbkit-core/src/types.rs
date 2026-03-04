#[derive(Debug, Clone, Copy, Default)]
pub struct NotLoaded;

#[derive(Debug, Clone, Copy, Default)]
pub struct HasMany<T>(std::marker::PhantomData<T>);

#[derive(Debug, Clone, Copy, Default)]
pub struct BelongsTo<T>(std::marker::PhantomData<T>);

#[derive(Debug, Clone, Copy, Default)]
pub struct ManyToMany<T>(std::marker::PhantomData<T>);

#[derive(Debug, Clone, PartialEq)]
pub struct PgVector<const N: usize> {
    values: [f32; N],
}

impl<const N: usize> PgVector<N> {
    pub fn new(values: [f32; N]) -> Result<Self, PgVectorError> {
        Self::validate_finite(values.as_slice())?;
        Ok(Self { values })
    }

    pub fn as_array(&self) -> &[f32; N] {
        &self.values
    }

    pub fn as_slice(&self) -> &[f32] {
        self.values.as_slice()
    }

    pub fn to_vec(&self) -> Vec<f32> {
        self.values.to_vec()
    }

    pub fn into_inner(self) -> [f32; N] {
        self.values
    }

    pub fn to_sql_literal(&self) -> String {
        vector_sql_literal(self.values.as_slice())
    }

    fn validate_finite(values: &[f32]) -> Result<(), PgVectorError> {
        for (idx, value) in values.iter().enumerate() {
            if !value.is_finite() {
                return Err(PgVectorError::NonFinite { index: idx });
            }
        }
        Ok(())
    }

    fn from_vec(values: Vec<f32>) -> Result<Self, PgVectorError> {
        if values.len() != N {
            return Err(PgVectorError::DimensionMismatch {
                expected: N,
                actual: values.len(),
            });
        }
        Self::validate_finite(&values)?;
        let values: [f32; N] =
            values
                .try_into()
                .map_err(|v: Vec<f32>| PgVectorError::DimensionMismatch {
                    expected: N,
                    actual: v.len(),
                })?;
        Ok(Self { values })
    }

    fn parse_text(input: &str) -> Result<Vec<f32>, PgVectorError> {
        let trimmed = input.trim();
        let inner = trimmed
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
            .ok_or_else(|| PgVectorError::InvalidTextFormat(trimmed.to_string()))?;

        if inner.trim().is_empty() {
            return Ok(Vec::new());
        }

        inner
            .split(',')
            .map(|part| {
                let value = part.trim();
                value
                    .parse::<f32>()
                    .map_err(|_| PgVectorError::InvalidTextFormat(trimmed.to_string()))
            })
            .collect()
    }

    #[cfg(feature = "sqlx")]
    fn decode_binary(bytes: &[u8]) -> Result<Self, PgVectorError> {
        if bytes.len() < 4 {
            return Err(PgVectorError::InvalidBinaryLength {
                expected_at_least: 4,
                actual: bytes.len(),
            });
        }

        let dims = i16::from_be_bytes([bytes[0], bytes[1]]);
        if dims < 0 {
            return Err(PgVectorError::InvalidBinaryLength {
                expected_at_least: 4,
                actual: bytes.len(),
            });
        }
        let dims = dims as usize;

        let expected = 4 + (dims * 4);
        if bytes.len() != expected {
            return Err(PgVectorError::InvalidBinaryLength {
                expected_at_least: expected,
                actual: bytes.len(),
            });
        }

        let mut values = Vec::with_capacity(dims);
        for idx in 0..dims {
            let start = 4 + (idx * 4);
            let bits = u32::from_be_bytes([
                bytes[start],
                bytes[start + 1],
                bytes[start + 2],
                bytes[start + 3],
            ]);
            values.push(f32::from_bits(bits));
        }

        Self::from_vec(values)
    }
}

impl<const N: usize> std::ops::Deref for PgVector<N> {
    type Target = [f32; N];

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl<const N: usize> AsRef<[f32]> for PgVector<N> {
    fn as_ref(&self) -> &[f32] {
        self.values.as_slice()
    }
}

impl<const N: usize> TryFrom<Vec<f32>> for PgVector<N> {
    type Error = PgVectorError;

    fn try_from(values: Vec<f32>) -> Result<Self, Self::Error> {
        Self::from_vec(values)
    }
}

impl<const N: usize> std::str::FromStr for PgVector<N> {
    type Err = PgVectorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_vec(Self::parse_text(s)?)
    }
}

impl<const N: usize> std::fmt::Display for PgVector<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_sql_literal().as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PgVectorError {
    DimensionMismatch {
        expected: usize,
        actual: usize,
    },
    NonFinite {
        index: usize,
    },
    InvalidTextFormat(String),
    InvalidBinaryLength {
        expected_at_least: usize,
        actual: usize,
    },
}

impl std::fmt::Display for PgVectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DimensionMismatch { expected, actual } => {
                write!(f, "pgvector dimension mismatch: expected {expected}, got {actual}")
            }
            Self::NonFinite { index } => write!(f, "pgvector contains non-finite value at index {index}"),
            Self::InvalidTextFormat(value) => write!(f, "invalid pgvector text format: {value}"),
            Self::InvalidBinaryLength {
                expected_at_least,
                actual,
            } => write!(
                f,
                "invalid pgvector binary length: expected at least {expected_at_least} bytes, got {actual}"
            ),
        }
    }
}

impl std::error::Error for PgVectorError {}

pub fn vector_sql_literal(values: &[f32]) -> String {
    let mut out = String::with_capacity(values.len().saturating_mul(8) + 2);
    out.push('[');
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(value.to_string().as_str());
    }
    out.push(']');
    out
}

#[cfg(feature = "sqlx")]
impl<const N: usize> sqlx::Type<sqlx::Postgres> for PgVector<N> {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("vector")
    }

    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
        *ty == sqlx::postgres::PgTypeInfo::with_name("vector")
            || <&str as sqlx::Type<sqlx::Postgres>>::compatible(ty)
    }
}

#[cfg(feature = "sqlx")]
impl<'r, const N: usize> sqlx::Decode<'r, sqlx::Postgres> for PgVector<N> {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        use sqlx::postgres::PgValueFormat;

        let vector = match value.format() {
            PgValueFormat::Text => <Self as std::str::FromStr>::from_str(value.as_str()?)?,
            PgValueFormat::Binary => Self::decode_binary(value.as_bytes()?)?,
        };
        Ok(vector)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveValue<T> {
    Unset,
    Set(T),
    Unchanged(T),
    UnchangedNull,
    Null,
}

impl<T> Default for ActiveValue<T> {
    fn default() -> Self {
        Self::Unset
    }
}

impl<T> ActiveValue<T> {
    pub fn set(&mut self, value: T) {
        *self = Self::Set(value);
    }

    pub fn unchanged(value: T) -> Self {
        Self::Unchanged(value)
    }

    pub fn unchanged_option(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Unchanged(value),
            None => Self::UnchangedNull,
        }
    }

    pub fn set_null(&mut self) {
        *self = Self::Null;
    }

    pub fn is_unset(&self) -> bool {
        matches!(self, Self::Unset)
    }

    pub fn is_unchanged(&self) -> bool {
        matches!(self, Self::Unchanged(_) | Self::UnchangedNull)
    }
}

impl<T> From<T> for ActiveValue<T> {
    fn from(value: T) -> Self {
        Self::Set(value)
    }
}

impl<T> From<Option<T>> for ActiveValue<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Set(value),
            None => Self::Null,
        }
    }
}

impl From<&str> for ActiveValue<String> {
    fn from(value: &str) -> Self {
        Self::Set(value.to_string())
    }
}
