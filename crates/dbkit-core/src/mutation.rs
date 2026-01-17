use std::marker::PhantomData;

use crate::compile::{CompiledSql, SqlBuilder, ToSql};
use crate::expr::{ColumnValue, Expr, Value};
use crate::schema::{Column, ColumnRef, Table};

#[derive(Debug, Clone)]
pub struct Insert<Out> {
    table: Table,
    columns: Vec<ColumnRef>,
    values: Vec<Value>,
    returning: Option<Vec<ColumnRef>>,
    returning_all: bool,
    _marker: PhantomData<Out>,
}

impl<Out> Insert<Out> {
    pub fn new(table: Table) -> Self {
        Self {
            table,
            columns: Vec::new(),
            values: Vec::new(),
            returning: None,
            returning_all: false,
            _marker: PhantomData,
        }
    }

    pub fn value<M, T, V>(mut self, column: Column<M, T>, value: V) -> Self
    where
        V: ColumnValue<T>,
    {
        let value = match value.into_value() {
            Some(value) => value,
            None => Value::Null,
        };
        self.columns.push(column.as_ref());
        self.values.push(value);
        self
    }

    pub fn returning(mut self, columns: Vec<ColumnRef>) -> Self {
        self.returning = Some(columns);
        self.returning_all = false;
        self
    }

    pub fn returning_all(mut self) -> Self {
        self.returning = None;
        self.returning_all = true;
        self
    }

    pub fn compile(&self) -> CompiledSql {
        let mut builder = SqlBuilder::new();
        builder.push_sql("INSERT INTO ");
        builder.push_sql(&self.table.qualified_name());
        builder.push_sql(" (");
        for (idx, col) in self.columns.iter().enumerate() {
            if idx > 0 {
                builder.push_sql(", ");
            }
            builder.push_sql(col.name);
        }
        builder.push_sql(") VALUES (");
        for (idx, value) in self.values.iter().enumerate() {
            if idx > 0 {
                builder.push_sql(", ");
            }
            builder.push_value(value.clone());
        }
        builder.push_sql(")");
        if self.returning_all {
            builder.push_sql(" RETURNING ");
            builder.push_sql(self.table.qualifier());
            builder.push_sql(".*");
        } else if let Some(columns) = &self.returning {
            builder.push_sql(" RETURNING ");
            for (idx, col) in columns.iter().enumerate() {
                if idx > 0 {
                    builder.push_sql(", ");
                }
                builder.push_column(*col);
            }
        }
        builder.finish()
    }
}

#[derive(Debug, Clone)]
pub struct Update<Out> {
    table: Table,
    sets: Vec<(ColumnRef, Value)>,
    filters: Vec<Expr<bool>>,
    returning: Option<Vec<ColumnRef>>,
    returning_all: bool,
    _marker: PhantomData<Out>,
}

impl<Out> Update<Out> {
    pub fn new(table: Table) -> Self {
        Self {
            table,
            sets: Vec::new(),
            filters: Vec::new(),
            returning: None,
            returning_all: false,
            _marker: PhantomData,
        }
    }

    pub fn set<M, T, V>(mut self, column: Column<M, T>, value: V) -> Self
    where
        V: ColumnValue<T>,
    {
        let value = match value.into_value() {
            Some(value) => value,
            None => Value::Null,
        };
        self.sets.push((column.as_ref(), value));
        self
    }

    pub fn filter(mut self, expr: Expr<bool>) -> Self {
        self.filters.push(expr);
        self
    }

    pub fn returning(mut self, columns: Vec<ColumnRef>) -> Self {
        self.returning = Some(columns);
        self.returning_all = false;
        self
    }

    pub fn returning_all(mut self) -> Self {
        self.returning = None;
        self.returning_all = true;
        self
    }

    pub fn compile(&self) -> CompiledSql {
        let mut builder = SqlBuilder::new();
        builder.push_sql("UPDATE ");
        builder.push_sql(&self.table.qualified_name());
        builder.push_sql(" SET ");
        for (idx, (col, value)) in self.sets.iter().enumerate() {
            if idx > 0 {
                builder.push_sql(", ");
            }
            builder.push_sql(col.name);
            builder.push_sql(" = ");
            builder.push_value(value.clone());
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
        if self.returning_all {
            builder.push_sql(" RETURNING ");
            builder.push_sql(self.table.qualifier());
            builder.push_sql(".*");
        } else if let Some(columns) = &self.returning {
            builder.push_sql(" RETURNING ");
            for (idx, col) in columns.iter().enumerate() {
                if idx > 0 {
                    builder.push_sql(", ");
                }
                builder.push_column(*col);
            }
        }
        builder.finish()
    }
}

#[derive(Debug, Clone)]
pub struct Delete {
    table: Table,
    filters: Vec<Expr<bool>>,
    returning: Option<Vec<ColumnRef>>,
    returning_all: bool,
}

impl Delete {
    pub fn new(table: Table) -> Self {
        Self {
            table,
            filters: Vec::new(),
            returning: None,
            returning_all: false,
        }
    }

    pub fn filter(mut self, expr: Expr<bool>) -> Self {
        self.filters.push(expr);
        self
    }

    pub fn returning(mut self, columns: Vec<ColumnRef>) -> Self {
        self.returning = Some(columns);
        self.returning_all = false;
        self
    }

    pub fn returning_all(mut self) -> Self {
        self.returning = None;
        self.returning_all = true;
        self
    }

    pub fn compile(&self) -> CompiledSql {
        let mut builder = SqlBuilder::new();
        builder.push_sql("DELETE FROM ");
        builder.push_sql(&self.table.qualified_name());
        if !self.filters.is_empty() {
            builder.push_sql(" WHERE ");
            for (idx, expr) in self.filters.iter().enumerate() {
                if idx > 0 {
                    builder.push_sql(" AND ");
                }
                expr.node.to_sql(&mut builder);
            }
        }
        if self.returning_all {
            builder.push_sql(" RETURNING ");
            builder.push_sql(self.table.qualifier());
            builder.push_sql(".*");
        } else if let Some(columns) = &self.returning {
            builder.push_sql(" RETURNING ");
            for (idx, col) in columns.iter().enumerate() {
                if idx > 0 {
                    builder.push_sql(", ");
                }
                builder.push_column(*col);
            }
        }
        builder.finish()
    }
}
