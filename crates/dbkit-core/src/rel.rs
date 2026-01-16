use std::marker::PhantomData;

use crate::expr::{BinaryOp, Expr, ExprNode};
use crate::schema::{ColumnRef, Table};

#[derive(Debug, Clone, Copy)]
pub enum RelationKind {
    HasMany,
    BelongsTo,
    ManyToMany,
}

#[derive(Debug, Clone, Copy)]
pub struct Relation {
    pub kind: RelationKind,
    pub parent: Table,
    pub child: Table,
    pub parent_key: ColumnRef,
    pub child_key: ColumnRef,
}

impl Relation {
    pub fn on_expr(&self) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.child_key)),
            op: BinaryOp::Eq,
            right: Box::new(ExprNode::Column(self.parent_key)),
        })
    }

    pub fn join_table(&self) -> Table {
        match self.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => self.child,
            RelationKind::BelongsTo => self.parent,
        }
    }
}

pub trait RelationInfo {
    type Parent;
    fn relation(&self) -> Relation;
}

pub trait BelongsToSpec<Parent> {
    const CHILD_TABLE: Table;
    const PARENT_TABLE: Table;
    const CHILD_KEY: ColumnRef;
    const PARENT_KEY: ColumnRef;
}

#[derive(Debug, Clone, Copy)]
pub struct HasMany<Parent, Child> {
    parent: Table,
    child: Table,
    parent_key: ColumnRef,
    child_key: ColumnRef,
    _marker: PhantomData<(Parent, Child)>,
}

impl<Parent, Child> HasMany<Parent, Child> {
    pub const fn new(
        parent: Table,
        child: Table,
        parent_key: ColumnRef,
        child_key: ColumnRef,
    ) -> Self {
        Self {
            parent,
            child,
            parent_key,
            child_key,
            _marker: PhantomData,
        }
    }

    pub fn selectin(self) -> crate::load::SelectIn<Self> {
        crate::load::SelectIn::new(self)
    }

    pub fn joined(self) -> crate::load::Joined<Self> {
        crate::load::Joined::new(self)
    }
}

impl<Parent, Child> RelationInfo for HasMany<Parent, Child> {
    type Parent = Parent;

    fn relation(&self) -> Relation {
        Relation {
            kind: RelationKind::HasMany,
            parent: self.parent,
            child: self.child,
            parent_key: self.parent_key,
            child_key: self.child_key,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BelongsTo<Child, Parent> {
    child: Table,
    parent: Table,
    child_key: ColumnRef,
    parent_key: ColumnRef,
    _marker: PhantomData<(Child, Parent)>,
}

impl<Child, Parent> BelongsTo<Child, Parent> {
    pub const fn new(
        child: Table,
        parent: Table,
        child_key: ColumnRef,
        parent_key: ColumnRef,
    ) -> Self {
        Self {
            child,
            parent,
            child_key,
            parent_key,
            _marker: PhantomData,
        }
    }

    pub fn selectin(self) -> crate::load::SelectIn<Self> {
        crate::load::SelectIn::new(self)
    }

    pub fn joined(self) -> crate::load::Joined<Self> {
        crate::load::Joined::new(self)
    }
}

impl<Child, Parent> RelationInfo for BelongsTo<Child, Parent> {
    type Parent = Child;

    fn relation(&self) -> Relation {
        Relation {
            kind: RelationKind::BelongsTo,
            parent: self.parent,
            child: self.child,
            parent_key: self.parent_key,
            child_key: self.child_key,
        }
    }
}
