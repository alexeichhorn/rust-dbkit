#[derive(Debug, Clone, Copy, Default)]
pub struct NotLoaded;

#[derive(Debug, Clone, Copy, Default)]
pub struct HasMany<T>(std::marker::PhantomData<T>);

#[derive(Debug, Clone, Copy, Default)]
pub struct BelongsTo<T>(std::marker::PhantomData<T>);

#[derive(Debug, Clone, Copy, Default)]
pub struct ManyToMany<T>(std::marker::PhantomData<T>);

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveValue<T> {
    Unset,
    Set(T),
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

    pub fn set_null(&mut self) {
        *self = Self::Null;
    }

    pub fn is_unset(&self) -> bool {
        matches!(self, Self::Unset)
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
