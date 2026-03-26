use crate::expr::{BinaryOp, BoolOp, ExprNode, IntervalField, UnaryOp, Value, VectorBinaryOp};
use crate::schema::ColumnRef;

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledSql {
    pub sql: String,
    pub binds: Vec<Value>,
}

#[derive(Debug, Default)]
pub struct SqlBuilder {
    sql: String,
    binds: Vec<Value>,
}

impl SqlBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_sql(&mut self, fragment: &str) {
        self.sql.push_str(fragment);
    }

    fn push_placeholder(&mut self, value: Value) {
        let idx = if let Some(existing) = self.binds.iter().position(|item| item == &value) {
            existing + 1
        } else {
            self.binds.push(value);
            self.binds.len()
        };
        self.sql.push('$');
        self.sql.push_str(&idx.to_string());
    }

    pub fn push_value(&mut self, value: Value) {
        if value == Value::Null {
            self.sql.push_str("NULL");
            return;
        }
        let cast_as_vector = matches!(&value, Value::Vector(_));
        let cast_as_interval = matches!(&value, Value::Interval(_));
        let cast_as_enum = match &value {
            Value::Enum { type_name, .. } => Some(*type_name),
            _ => None,
        };
        self.push_placeholder(value);
        if cast_as_vector {
            self.sql.push_str("::vector");
        } else if cast_as_interval {
            self.sql.push_str("::interval");
        } else if let Some(type_name) = cast_as_enum {
            self.sql.push_str("::");
            self.sql.push_str(type_name);
        }
    }

    pub fn push_column(&mut self, col: ColumnRef) {
        self.sql.push_str(&col.qualified_name());
    }

    pub fn push_compiled_sql(&mut self, compiled: &CompiledSql) {
        let bytes = compiled.sql.as_bytes();
        let mut idx = 0;
        let mut segment_start = 0;

        while idx < bytes.len() {
            if bytes[idx] == b'$' {
                // Scan bytewise and only interpret ASCII placeholder syntax (`$` + digits).
                // Everything else is copied through verbatim below as UTF-8 string slices.
                let prev_is_ident = idx > 0 && is_bind_ident_char(bytes[idx - 1]);
                let start = idx + 1;
                let mut end = start;
                while end < bytes.len() && bytes[end].is_ascii_digit() {
                    end += 1;
                }
                let next_is_ident = end < bytes.len() && is_bind_ident_char(bytes[end]);

                if end > start && !prev_is_ident && !next_is_ident {
                    self.push_sql(&compiled.sql[segment_start..idx]);
                    let bind_idx = compiled.sql[start..end].parse::<usize>().expect("valid bind index");
                    let value = compiled.binds[bind_idx - 1].clone();
                    // Rebind only the placeholder token. Any suffix text such as `::vector`,
                    // `::interval`, or `::schema.enum_type` remains in `compiled.sql` and is
                    // copied verbatim by the fallback branch after this placeholder is emitted.
                    self.push_placeholder(value);
                    idx = end;
                    segment_start = end;
                    continue;
                }
            }

            idx += 1;
        }

        self.push_sql(&compiled.sql[segment_start..]);
    }

    pub fn finish(self) -> CompiledSql {
        CompiledSql {
            sql: self.sql,
            binds: self.binds,
        }
    }
}

fn is_bind_ident_char(byte: u8) -> bool {
    !byte.is_ascii() || byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

pub trait ToSql {
    fn to_sql(&self, builder: &mut SqlBuilder);
}

impl ToSql for ExprNode {
    fn to_sql(&self, builder: &mut SqlBuilder) {
        match self {
            ExprNode::Column(col) => builder.push_column(*col),
            ExprNode::Value(value) => builder.push_value(value.clone()),
            ExprNode::Func { name, args } => {
                builder.push_sql(name);
                builder.push_sql("(");
                for (idx, arg) in args.iter().enumerate() {
                    if idx > 0 {
                        builder.push_sql(", ");
                    }
                    arg.to_sql(builder);
                }
                builder.push_sql(")");
            }
            ExprNode::VectorBinary { left, op, right } => {
                builder.push_sql("(");
                left.to_sql(builder);
                builder.push_sql(match op {
                    VectorBinaryOp::L2Distance => " <-> ",
                    VectorBinaryOp::CosineDistance => " <=> ",
                    VectorBinaryOp::InnerProductDistance => " <#> ",
                    VectorBinaryOp::L1Distance => " <+> ",
                });
                right.to_sql(builder);
                builder.push_sql(")");
            }
            ExprNode::MakeInterval { field, value } => {
                builder.push_sql("MAKE_INTERVAL(");
                builder.push_sql(match field {
                    IntervalField::Days => "days => ",
                    IntervalField::Hours => "hours => ",
                    IntervalField::Minutes => "mins => ",
                    IntervalField::Seconds => "secs => ",
                });
                value.to_sql(builder);
                builder.push_sql(")");
            }
            ExprNode::Binary { left, op, right } => {
                builder.push_sql("(");
                left.to_sql(builder);
                builder.push_sql(match op {
                    BinaryOp::Add => " + ",
                    BinaryOp::Sub => " - ",
                    BinaryOp::Mul => " * ",
                    BinaryOp::Eq => " = ",
                    BinaryOp::Ne => " <> ",
                    BinaryOp::IsDistinctFrom => " IS DISTINCT FROM ",
                    BinaryOp::IsNotDistinctFrom => " IS NOT DISTINCT FROM ",
                    BinaryOp::Lt => " < ",
                    BinaryOp::Le => " <= ",
                    BinaryOp::Gt => " > ",
                    BinaryOp::Ge => " >= ",
                });
                right.to_sql(builder);
                builder.push_sql(")");
            }
            ExprNode::Bool { left, op, right } => {
                builder.push_sql("(");
                left.to_sql(builder);
                builder.push_sql(match op {
                    BoolOp::And => " AND ",
                    BoolOp::Or => " OR ",
                });
                right.to_sql(builder);
                builder.push_sql(")");
            }
            ExprNode::Unary { op, expr } => {
                builder.push_sql(match op {
                    UnaryOp::Not => "NOT ",
                });
                builder.push_sql("(");
                expr.to_sql(builder);
                builder.push_sql(")");
            }
            ExprNode::In { expr, values } => {
                if values.is_empty() {
                    builder.push_sql("(FALSE)");
                    return;
                }
                builder.push_sql("(");
                expr.to_sql(builder);
                builder.push_sql(" IN (");
                for (idx, value) in values.iter().enumerate() {
                    if idx > 0 {
                        builder.push_sql(", ");
                    }
                    builder.push_value(value.clone());
                }
                builder.push_sql("))");
            }
            ExprNode::IsNull { expr, negated } => {
                builder.push_sql("(");
                expr.to_sql(builder);
                if *negated {
                    builder.push_sql(" IS NOT NULL)");
                } else {
                    builder.push_sql(" IS NULL)");
                }
            }
            ExprNode::Like {
                expr,
                pattern,
                case_insensitive,
            } => {
                builder.push_sql("(");
                expr.to_sql(builder);
                builder.push_sql(if *case_insensitive { " ILIKE " } else { " LIKE " });
                builder.push_value(pattern.clone());
                builder.push_sql(")");
            }
            ExprNode::Exists { subquery } => {
                builder.push_sql("EXISTS (");
                builder.push_compiled_sql(subquery);
                builder.push_sql(")");
            }
        }
    }
}
