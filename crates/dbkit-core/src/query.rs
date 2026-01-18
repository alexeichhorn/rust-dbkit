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

#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub column: ColumnRef,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone)]
pub struct Select<Out, Loads = NoLoad> {
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
    loads: Loads,
    _marker: PhantomData<Out>,
}

impl<Out> Select<Out, NoLoad> {
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
            loads: NoLoad,
            _marker: PhantomData,
        }
    }
}

impl<Out, Loads> Select<Out, Loads> {
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

    pub fn into_model<T>(self) -> Select<T, Loads> {
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
            loads: self.loads,
            _marker: PhantomData,
        }
    }

    pub fn with<L>(self, load: L) -> Select<L::Out2, LoadChain<Loads, L>>
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
            loads: LoadChain {
                prev: self.loads,
                load,
            },
            _marker: PhantomData,
        }
    }

    pub fn compile(&self) -> CompiledSql {
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
            }
            None => {
                builder.push_sql(self.table.qualifier());
                builder.push_sql(".*");
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
        if !self.order_by.is_empty() {
            builder.push_sql(" ORDER BY ");
            for (idx, order) in self.order_by.iter().enumerate() {
                if idx > 0 {
                    builder.push_sql(", ");
                }
                builder.push_column(order.column);
                builder.push_sql(match order.direction {
                    OrderDirection::Asc => " ASC",
                    OrderDirection::Desc => " DESC",
                });
            }
        }
        if let Some(limit) = self.limit {
            builder.push_sql(" LIMIT ");
            builder.push_sql(&limit.to_string());
        }
        if let Some(offset) = self.offset {
            builder.push_sql(" OFFSET ");
            builder.push_sql(&offset.to_string());
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
}

impl Order {
    pub fn asc(column: ColumnRef) -> Self {
        Self {
            column,
            direction: OrderDirection::Asc,
        }
    }

    pub fn desc(column: ColumnRef) -> Self {
        Self {
            column,
            direction: OrderDirection::Desc,
        }
    }
}
