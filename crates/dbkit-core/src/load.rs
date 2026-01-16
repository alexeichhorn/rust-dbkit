use crate::rel::{Relation, RelationInfo};
use crate::Select;

#[derive(Debug, Clone, Copy)]
pub enum Strategy {
    SelectIn,
    Joined,
}

#[derive(Debug, Clone)]
pub struct LoadSpec {
    pub relation: Relation,
    pub strategy: Strategy,
    pub nested: Vec<LoadSpec>,
}

#[derive(Debug, Clone, Copy)]
pub struct NoLoad;

pub trait NestedLoad {
    fn into_nested(self) -> Vec<LoadSpec>;
}

impl NestedLoad for NoLoad {
    fn into_nested(self) -> Vec<LoadSpec> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SelectIn<R, Nested = NoLoad> {
    pub rel: R,
    pub nested: Nested,
}

impl<R> SelectIn<R, NoLoad> {
    pub fn new(rel: R) -> Self {
        Self {
            rel,
            nested: NoLoad,
        }
    }
}

impl<R, Nested> SelectIn<R, Nested> {
    pub fn with<N>(self, nested: N) -> SelectIn<R, N> {
        SelectIn {
            rel: self.rel,
            nested,
        }
    }

    pub fn into_spec(self) -> LoadSpec
    where
        R: RelationInfo,
        Nested: NestedLoad,
    {
        LoadSpec {
            relation: self.rel.relation(),
            strategy: Strategy::SelectIn,
            nested: self.nested.into_nested(),
        }
    }
}

impl<R, Nested> NestedLoad for SelectIn<R, Nested>
where
    R: RelationInfo,
    Nested: NestedLoad,
{
    fn into_nested(self) -> Vec<LoadSpec> {
        vec![self.into_spec()]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Joined<R, Nested = NoLoad> {
    pub rel: R,
    pub nested: Nested,
}

impl<R> Joined<R, NoLoad> {
    pub fn new(rel: R) -> Self {
        Self {
            rel,
            nested: NoLoad,
        }
    }
}

impl<R, Nested> Joined<R, Nested> {
    pub fn with<N>(self, nested: N) -> Joined<R, N> {
        Joined {
            rel: self.rel,
            nested,
        }
    }

    pub fn into_spec(self) -> LoadSpec
    where
        R: RelationInfo,
        Nested: NestedLoad,
    {
        LoadSpec {
            relation: self.rel.relation(),
            strategy: Strategy::Joined,
            nested: self.nested.into_nested(),
        }
    }
}

impl<R, Nested> NestedLoad for Joined<R, Nested>
where
    R: RelationInfo,
    Nested: NestedLoad,
{
    fn into_nested(self) -> Vec<LoadSpec> {
        vec![self.into_spec()]
    }
}

pub trait ApplyLoad<Out> {
    type Out2;
    fn apply(self, select: Select<Out>) -> Select<Self::Out2>;
}

impl<Out> ApplyLoad<Out> for NoLoad {
    type Out2 = Out;

    fn apply(self, select: Select<Out>) -> Select<Self::Out2> {
        select
    }
}
