use std::marker::PhantomData;

use crate::expr::{BinaryOp, Expr, ExprNode};
use crate::schema::{ColumnRef, Table};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub join_table: Option<Table>,
    pub join_parent_key: Option<ColumnRef>,
    pub join_child_key: Option<ColumnRef>,
}

impl Relation {
    pub fn on_expr(&self) -> Expr<bool> {
        let left = match self.kind {
            RelationKind::ManyToMany => self.join_parent_key.unwrap_or(self.child_key),
            _ => self.child_key,
        };
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(left)),
            op: BinaryOp::Eq,
            right: Box::new(ExprNode::Column(self.parent_key)),
        })
    }

    pub fn join_table(&self) -> Table {
        match self.kind {
            RelationKind::HasMany => self.child,
            RelationKind::ManyToMany => self.join_table.unwrap_or(self.child),
            RelationKind::BelongsTo => self.parent,
        }
    }

    pub fn join_steps(&self) -> Vec<(Table, Expr<bool>)> {
        if self.kind == RelationKind::ManyToMany {
            let join_table = self.join_table.expect("many-to-many join table");
            let join_parent_key = self.join_parent_key.expect("many-to-many join parent key");
            let join_child_key = self.join_child_key.expect("many-to-many join child key");

            let first = Expr::new(ExprNode::Binary {
                left: Box::new(ExprNode::Column(join_parent_key)),
                op: BinaryOp::Eq,
                right: Box::new(ExprNode::Column(self.parent_key)),
            });
            let second = Expr::new(ExprNode::Binary {
                left: Box::new(ExprNode::Column(join_child_key)),
                op: BinaryOp::Eq,
                right: Box::new(ExprNode::Column(self.child_key)),
            });
            vec![(join_table, first), (self.child, second)]
        } else {
            vec![(self.join_table(), self.on_expr())]
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
            join_table: None,
            join_parent_key: None,
            join_child_key: None,
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
            join_table: None,
            join_parent_key: None,
            join_child_key: None,
        }
    }
}

pub trait ManyToManyThrough {
    type Through;
}

#[derive(Debug, Clone, Copy)]
pub struct ManyToMany<Parent, Child, Through> {
    parent: Table,
    child: Table,
    join: Table,
    parent_key: ColumnRef,
    child_key: ColumnRef,
    join_parent_key: ColumnRef,
    join_child_key: ColumnRef,
    _marker: PhantomData<(Parent, Child, Through)>,
}

impl<Parent, Child, Through> ManyToMany<Parent, Child, Through> {
    pub const fn new(
        parent: Table,
        child: Table,
        join: Table,
        parent_key: ColumnRef,
        child_key: ColumnRef,
        join_parent_key: ColumnRef,
        join_child_key: ColumnRef,
    ) -> Self {
        Self {
            parent,
            child,
            join,
            parent_key,
            child_key,
            join_parent_key,
            join_child_key,
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

impl<Parent, Child, Through> RelationInfo for ManyToMany<Parent, Child, Through> {
    type Parent = Parent;

    fn relation(&self) -> Relation {
        Relation {
            kind: RelationKind::ManyToMany,
            parent: self.parent,
            child: self.child,
            parent_key: self.parent_key,
            child_key: self.child_key,
            join_table: Some(self.join),
            join_parent_key: Some(self.join_parent_key),
            join_child_key: Some(self.join_child_key),
        }
    }
}

impl<Parent, Child, Through> ManyToManyThrough for ManyToMany<Parent, Child, Through> {
    type Through = Through;
}
