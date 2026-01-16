use std::marker::PhantomData;

use crate::compile::{CompiledSql, SqlBuilder, ToSql};
use crate::expr::Expr;
use crate::load::{ApplyLoad, LoadSpec};
use crate::rel::RelationInfo;
use crate::schema::{ColumnRef, Table};

#[derive(Debug, Clone, Copy)]
pub enum JoinKind {
    Inner,
    Left,
}

#[derive(Debug, Clone, Copy)]
pub struct Join {
    pub table: Table,
    pub on: Expr<bool>,
    pub kind: JoinKind,
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
pub struct Select<Out> {
    table: Table,
    columns: Option<Vec<ColumnRef>>,
    joins: Vec<Join>,
    filters: Vec<Expr<bool>>,
    order_by: Vec<Order>,
    limit: Option<u64>,
    offset: Option<u64>,
    distinct: bool,
    loads: Vec<LoadSpec>,
    _marker: PhantomData<Out>,
}

impl<Out> Select<Out> {
    pub fn new(table: Table) -> Self {
        Self {
            table,
            columns: None,
            joins: Vec::new(),
            filters: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            distinct: false,
            loads: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn filter(mut self, expr: Expr<bool>) -> Self {
        self.filters.push(expr);
        self
    }

    pub fn join<R>(mut self, rel: R) -> Self
    where
        R: RelationInfo<Parent = Out>,
    {
        let relation = rel.relation();
        self.joins.push(Join {
            table: relation.join_table(),
            on: relation.on_expr(),
            kind: JoinKind::Inner,
        });
        self
    }

    pub fn left_join<R>(mut self, rel: R) -> Self
    where
        R: RelationInfo<Parent = Out>,
    {
        let relation = rel.relation();
        self.joins.push(Join {
            table: relation.join_table(),
            on: relation.on_expr(),
            kind: JoinKind::Left,
        });
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
        self.columns = Some(columns);
        self
    }

    pub fn with<L>(self, load: L) -> Select<L::Out2>
    where
        L: ApplyLoad<Out>,
    {
        load.apply(self)
    }

    #[doc(hidden)]
    pub fn push_load(mut self, load: LoadSpec) -> Self {
        self.loads.push(load);
        self
    }

    #[doc(hidden)]
    pub fn into_output<Out2>(self) -> Select<Out2> {
        Select {
            table: self.table,
            columns: self.columns,
            joins: self.joins,
            filters: self.filters,
            order_by: self.order_by,
            limit: self.limit,
            offset: self.offset,
            distinct: self.distinct,
            loads: self.loads,
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
                    builder.push_column(*col);
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
