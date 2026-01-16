#[derive(Debug, Clone, Copy)]
pub struct NoLoad;

#[derive(Debug, Clone, Copy)]
pub struct LoadChain<Prev, L> {
    pub prev: Prev,
    pub load: L,
}

pub trait ApplyLoad<Out> {
    type Out2;
}

impl<Out> ApplyLoad<Out> for NoLoad {
    type Out2 = Out;
}

impl<Out, Prev, L> ApplyLoad<Out> for LoadChain<Prev, L>
where
    Prev: ApplyLoad<Out>,
    L: ApplyLoad<Prev::Out2>,
{
    type Out2 = <L as ApplyLoad<Prev::Out2>>::Out2;
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
    pub fn with<L>(self, load: L) -> SelectIn<R, LoadChain<Nested, L>> {
        SelectIn {
            rel: self.rel,
            nested: LoadChain {
                prev: self.nested,
                load,
            },
        }
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
    pub fn with<L>(self, load: L) -> Joined<R, LoadChain<Nested, L>> {
        Joined {
            rel: self.rel,
            nested: LoadChain {
                prev: self.nested,
                load,
            },
        }
    }
}
