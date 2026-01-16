#[derive(Debug, Clone, Copy, Default)]
pub struct NotLoaded;

#[derive(Debug, Clone, Copy, Default)]
pub struct HasMany<T>(std::marker::PhantomData<T>);

#[derive(Debug, Clone, Copy, Default)]
pub struct BelongsTo<T>(std::marker::PhantomData<T>);

#[derive(Debug, Clone, Copy, Default)]
pub struct ManyToMany<T>(std::marker::PhantomData<T>);
