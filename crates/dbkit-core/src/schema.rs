use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Table {
    pub name: &'static str,
    pub schema: Option<&'static str>,
    pub alias: Option<&'static str>,
}

impl Table {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            schema: None,
            alias: None,
        }
    }

    pub const fn with_schema(mut self, schema: &'static str) -> Self {
        self.schema = Some(schema);
        self
    }

    pub const fn with_alias(mut self, alias: &'static str) -> Self {
        self.alias = Some(alias);
        self
    }

    pub fn qualifier(&self) -> &'static str {
        if let Some(alias) = self.alias {
            alias
        } else {
            self.name
        }
    }

    pub fn qualified_name(&self) -> String {
        match self.schema {
            Some(schema) => format!("{}.{}", schema, self.name),
            None => self.name.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnRef {
    pub table: Table,
    pub name: &'static str,
}

impl ColumnRef {
    pub const fn new(table: Table, name: &'static str) -> Self {
        Self { table, name }
    }

    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.table.qualifier(), self.name)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Column<M, T> {
    pub table: Table,
    pub name: &'static str,
    _marker: PhantomData<(M, T)>,
}

impl<M, T> Column<M, T> {
    pub const fn new(table: Table, name: &'static str) -> Self {
        Self {
            table,
            name,
            _marker: PhantomData,
        }
    }

    pub const fn as_ref(&self) -> ColumnRef {
        ColumnRef::new(self.table, self.name)
    }
}
