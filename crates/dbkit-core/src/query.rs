use std::marker::PhantomData;

use crate::compile::{CompiledSql, SqlBuilder, ToSql};
use crate::expr::{Expr, ExprNode, IntoExpr};
use crate::load::{ApplyLoad, LoadChain, NoLoad};
use crate::rel::RelationInfo;
use crate::schema::{ColumnRef, Table};

#[derive(Debug, Clone, Copy)]
pub enum JoinKind {
    Inner,
    Left,
}

#[derive(Debug, Clone)]
pub struct Join {
    pub table: Table,
    pub on: Expr<bool>,
    pub kind: JoinKind,
}

#[derive(Debug, Clone)]
pub struct SelectItem {
    pub expr: ExprNode,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub enum OrderExpr {
    Expr(ExprNode),
    Alias(String),
}

pub trait IntoOrderExpr {
    fn into_order_expr(self) -> OrderExpr;
}

impl IntoOrderExpr for ColumnRef {
    fn into_order_expr(self) -> OrderExpr {
        OrderExpr::Expr(ExprNode::Column(self))
    }
}

impl<M, T> IntoOrderExpr for crate::schema::Column<M, T> {
    fn into_order_expr(self) -> OrderExpr {
        OrderExpr::Expr(ExprNode::Column(self.as_ref()))
    }
}

impl<T> IntoOrderExpr for Expr<T> {
    fn into_order_expr(self) -> OrderExpr {
        OrderExpr::Expr(self.node)
    }
}

#[derive(Debug, Clone)]
pub struct Order {
    pub expr: OrderExpr,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoRowLock;

#[derive(Debug, Clone, Copy, Default)]
pub struct ForUpdateRowLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowLockWait {
    Wait,
    SkipLocked,
    NoWait,
}

#[derive(Debug, Clone)]
pub struct Select<Out, Loads = NoLoad, Lock = NoRowLock> {
    table: Table,
    columns: Option<Vec<SelectItem>>,
    joins: Vec<Join>,
    filters: Vec<Expr<bool>>,
    group_by: Vec<ExprNode>,
    having: Vec<Expr<bool>>,
    order_by: Vec<Order>,
    limit: Option<u64>,
    offset: Option<u64>,
    distinct: bool,
    row_lock_wait: Option<RowLockWait>,
    loads: Loads,
    _marker: PhantomData<Out>,
    _lock_marker: PhantomData<Lock>,
}

impl<Out> Select<Out, NoLoad, NoRowLock> {
    pub fn new(table: Table) -> Self {
        Self {
            table,
            columns: None,
            joins: Vec::new(),
            filters: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            distinct: false,
            row_lock_wait: None,
            loads: NoLoad,
            _marker: PhantomData,
            _lock_marker: PhantomData,
        }
    }
}

impl<Out, Loads, Lock> Select<Out, Loads, Lock> {
    pub fn table(&self) -> Table {
        self.table
    }

    pub fn columns_ref(&self) -> Option<&[SelectItem]> {
        self.columns.as_deref()
    }

    pub fn joins(&self) -> &[Join] {
        &self.joins
    }

    pub fn select_only(mut self) -> Self {
        self.columns = Some(Vec::new());
        self
    }

    pub fn column<T>(mut self, expr: impl IntoExpr<T>) -> Self {
        let item = SelectItem {
            expr: expr.into_expr().node,
            alias: None,
        };
        match &mut self.columns {
            Some(columns) => columns.push(item),
            None => self.columns = Some(vec![item]),
        }
        self
    }

    pub fn column_as<T>(mut self, expr: impl IntoExpr<T>, alias: &str) -> Self {
        let item = SelectItem {
            expr: expr.into_expr().node,
            alias: Some(alias.to_string()),
        };
        match &mut self.columns {
            Some(columns) => columns.push(item),
            None => self.columns = Some(vec![item]),
        }
        self
    }

    pub fn filter(mut self, expr: Expr<bool>) -> Self {
        self.filters.push(expr);
        self
    }

    pub fn group_by<T>(mut self, expr: impl IntoExpr<T>) -> Self {
        self.group_by.push(expr.into_expr().node);
        self
    }

    pub fn having(mut self, expr: Expr<bool>) -> Self {
        self.having.push(expr);
        self
    }

    pub fn join<R>(mut self, rel: R) -> Self
    where
        R: RelationInfo<Parent = Out>,
    {
        let relation = rel.relation();
        for (table, on) in relation.join_steps() {
            self.joins.push(Join {
                table,
                on,
                kind: JoinKind::Inner,
            });
        }
        self
    }

    pub fn left_join<R>(mut self, rel: R) -> Self
    where
        R: RelationInfo<Parent = Out>,
    {
        let relation = rel.relation();
        for (table, on) in relation.join_steps() {
            self.joins.push(Join {
                table,
                on,
                kind: JoinKind::Left,
            });
        }
        self
    }

    pub fn join_on(mut self, table: Table, on: Expr<bool>) -> Self {
        self.joins.push(Join {
            table,
            on,
            kind: JoinKind::Inner,
        });
        self
    }

    pub fn left_join_on(mut self, table: Table, on: Expr<bool>) -> Self {
        self.joins.push(Join {
            table,
            on,
            kind: JoinKind::Left,
        });
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    pub fn order_by(mut self, order: Order) -> Self {
        self.order_by.push(order);
        self
    }

    pub fn columns(mut self, columns: Vec<ColumnRef>) -> Self {
        let items = columns
            .into_iter()
            .map(|col| SelectItem {
                expr: ExprNode::Column(col),
                alias: None,
            })
            .collect::<Vec<_>>();
        self.columns = Some(items);
        self
    }

    pub fn into_model<T>(self) -> Select<T, Loads, Lock> {
        Select {
            table: self.table,
            columns: self.columns,
            joins: self.joins,
            filters: self.filters,
            group_by: self.group_by,
            having: self.having,
            order_by: self.order_by,
            limit: self.limit,
            offset: self.offset,
            distinct: self.distinct,
            row_lock_wait: self.row_lock_wait,
            loads: self.loads,
            _marker: PhantomData,
            _lock_marker: PhantomData,
        }
    }

    pub fn with<L>(self, load: L) -> Select<L::Out2, LoadChain<Loads, L>, Lock>
    where
        L: ApplyLoad<Out>,
    {
        Select {
            table: self.table,
            columns: self.columns,
            joins: self.joins,
            filters: self.filters,
            group_by: self.group_by,
            having: self.having,
            order_by: self.order_by,
            limit: self.limit,
            offset: self.offset,
            distinct: self.distinct,
            row_lock_wait: self.row_lock_wait,
            loads: LoadChain {
                prev: self.loads,
                load,
            },
            _marker: PhantomData,
            _lock_marker: PhantomData,
        }
    }

    pub fn for_update(self) -> Select<Out, Loads, ForUpdateRowLock> {
        Select {
            table: self.table,
            columns: self.columns,
            joins: self.joins,
            filters: self.filters,
            group_by: self.group_by,
            having: self.having,
            order_by: self.order_by,
            limit: self.limit,
            offset: self.offset,
            distinct: self.distinct,
            row_lock_wait: Some(self.row_lock_wait.unwrap_or(RowLockWait::Wait)),
            loads: self.loads,
            _marker: PhantomData,
            _lock_marker: PhantomData,
        }
    }

    pub fn compile(&self) -> CompiledSql {
        self.compile_inner(true, true, true)
    }

    pub fn compile_without_pagination(&self) -> CompiledSql {
        self.compile_inner(false, false, false)
    }

    pub fn compile_with_extra(
        &self,
        extra_columns: &[SelectItem],
        extra_joins: &[Join],
    ) -> CompiledSql {
        self.compile_inner_with(extra_columns, extra_joins, true, true, true)
    }

    fn compile_inner(
        &self,
        include_order: bool,
        include_pagination: bool,
        include_locking: bool,
    ) -> CompiledSql {
        self.compile_inner_with(&[], &[], include_order, include_pagination, include_locking)
    }

    fn compile_inner_with(
        &self,
        extra_columns: &[SelectItem],
        extra_joins: &[Join],
        include_order: bool,
        include_pagination: bool,
        include_locking: bool,
    ) -> CompiledSql {
        let mut builder = SqlBuilder::new();
        builder.push_sql("SELECT ");
        if self.distinct {
            builder.push_sql("DISTINCT ");
        }
        match &self.columns {
            Some(columns) => {
                for (idx, col) in columns.iter().enumerate() {
                    if idx > 0 {
                        builder.push_sql(", ");
                    }
                    col.expr.to_sql(&mut builder);
                    if let Some(alias) = &col.alias {
                        builder.push_sql(" AS ");
                        builder.push_sql(alias);
                    }
                }
                if !extra_columns.is_empty() {
                    for col in extra_columns {
                        builder.push_sql(", ");
                        col.expr.to_sql(&mut builder);
                        if let Some(alias) = &col.alias {
                            builder.push_sql(" AS ");
                            builder.push_sql(alias);
                        }
                    }
                }
            }
            None => {
                builder.push_sql(self.table.qualifier());
                builder.push_sql(".*");
                if !extra_columns.is_empty() {
                    for col in extra_columns {
                        builder.push_sql(", ");
                        col.expr.to_sql(&mut builder);
                        if let Some(alias) = &col.alias {
                            builder.push_sql(" AS ");
                            builder.push_sql(alias);
                        }
                    }
                }
            }
        }
        builder.push_sql(" FROM ");
        builder.push_sql(&self.table.qualified_name());
        if let Some(alias) = self.table.alias {
            builder.push_sql(" ");
            builder.push_sql(alias);
        }
        for join in &self.joins {
            builder.push_sql(match join.kind {
                JoinKind::Inner => " JOIN ",
                JoinKind::Left => " LEFT JOIN ",
            });
            builder.push_sql(&join.table.qualified_name());
            if let Some(alias) = join.table.alias {
                builder.push_sql(" ");
                builder.push_sql(alias);
            }
            builder.push_sql(" ON ");
            join.on.node.to_sql(&mut builder);
        }
        for join in extra_joins {
            builder.push_sql(match join.kind {
                JoinKind::Inner => " JOIN ",
                JoinKind::Left => " LEFT JOIN ",
            });
            builder.push_sql(&join.table.qualified_name());
            if let Some(alias) = join.table.alias {
                builder.push_sql(" ");
                builder.push_sql(alias);
            }
            builder.push_sql(" ON ");
            join.on.node.to_sql(&mut builder);
        }
        if !self.filters.is_empty() {
            builder.push_sql(" WHERE ");
            for (idx, expr) in self.filters.iter().enumerate() {
                if idx > 0 {
                    builder.push_sql(" AND ");
                }
                expr.node.to_sql(&mut builder);
            }
        }
        if !self.group_by.is_empty() {
            builder.push_sql(" GROUP BY ");
            for (idx, expr) in self.group_by.iter().enumerate() {
                if idx > 0 {
                    builder.push_sql(", ");
                }
                expr.to_sql(&mut builder);
            }
        }
        if !self.having.is_empty() {
            builder.push_sql(" HAVING ");
            for (idx, expr) in self.having.iter().enumerate() {
                if idx > 0 {
                    builder.push_sql(" AND ");
                }
                expr.node.to_sql(&mut builder);
            }
        }
        if include_order && !self.order_by.is_empty() {
            builder.push_sql(" ORDER BY ");
            for (idx, order) in self.order_by.iter().enumerate() {
                if idx > 0 {
                    builder.push_sql(", ");
                }
                match &order.expr {
                    OrderExpr::Expr(expr) => expr.to_sql(&mut builder),
                    OrderExpr::Alias(alias) => builder.push_sql(alias),
                }
                builder.push_sql(match order.direction {
                    OrderDirection::Asc => " ASC",
                    OrderDirection::Desc => " DESC",
                });
            }
        }
        if include_pagination {
            if let Some(limit) = self.limit {
                builder.push_sql(" LIMIT ");
                builder.push_sql(&limit.to_string());
            }
            if let Some(offset) = self.offset {
                builder.push_sql(" OFFSET ");
                builder.push_sql(&offset.to_string());
            }
        }
        if include_locking {
            if let Some(wait) = self.row_lock_wait {
                builder.push_sql(" FOR UPDATE");
                match wait {
                    RowLockWait::Wait => {}
                    RowLockWait::SkipLocked => builder.push_sql(" SKIP LOCKED"),
                    RowLockWait::NoWait => builder.push_sql(" NOWAIT"),
                }
            }
        }
        builder.finish()
    }

    pub fn debug_sql(&self) -> String {
        self.compile().sql
    }

    pub fn into_parts(self) -> (CompiledSql, Loads) {
        let compiled = self.compile();
        (compiled, self.loads)
    }

    pub fn into_parts_with_loads(self) -> (Select<Out, NoLoad, Lock>, Loads) {
        let Select {
            table,
            columns,
            joins,
            filters,
            group_by,
            having,
            order_by,
            limit,
            offset,
            distinct,
            row_lock_wait,
            loads,
            _marker,
            _lock_marker,
        } = self;

        let select = Select {
            table,
            columns,
            joins,
            filters,
            group_by,
            having,
            order_by,
            limit,
            offset,
            distinct,
            row_lock_wait,
            loads: NoLoad,
            _marker,
            _lock_marker: PhantomData,
        };

        (select, loads)
    }
}

impl<Out, Loads> Select<Out, Loads, ForUpdateRowLock> {
    pub fn skip_locked(mut self) -> Self {
        self.row_lock_wait = Some(RowLockWait::SkipLocked);
        self
    }

    pub fn nowait(mut self) -> Self {
        self.row_lock_wait = Some(RowLockWait::NoWait);
        self
    }
}

impl Order {
    pub fn asc(expr: impl IntoOrderExpr) -> Self {
        Self {
            expr: expr.into_order_expr(),
            direction: OrderDirection::Asc,
        }
    }

    pub fn desc(expr: impl IntoOrderExpr) -> Self {
        Self {
            expr: expr.into_order_expr(),
            direction: OrderDirection::Desc,
        }
    }

    pub fn asc_alias(alias: &str) -> Self {
        Self {
            expr: OrderExpr::Alias(alias.to_string()),
            direction: OrderDirection::Asc,
        }
    }

    pub fn desc_alias(alias: &str) -> Self {
        Self {
            expr: OrderExpr::Alias(alias.to_string()),
            direction: OrderDirection::Desc,
        }
    }
}
