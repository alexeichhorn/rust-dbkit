use crate::expr::{BinaryOp, BoolOp, ExprNode, UnaryOp, Value};
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

    pub fn push_value(&mut self, value: Value) {
        if value == Value::Null {
            self.sql.push_str("NULL");
            return;
        }
        let cast_as_vector = matches!(value, Value::Vector(_));
        let idx = if let Some(existing) = self.binds.iter().position(|item| item == &value) {
            existing + 1
        } else {
            self.binds.push(value);
            self.binds.len()
        };
        self.sql.push('$');
        self.sql.push_str(&idx.to_string());
        if cast_as_vector {
            self.sql.push_str("::vector");
        }
    }

    pub fn push_column(&mut self, col: ColumnRef) {
        self.sql.push_str(&col.qualified_name());
    }

    pub fn finish(self) -> CompiledSql {
        CompiledSql {
            sql: self.sql,
            binds: self.binds,
        }
    }
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
            ExprNode::Binary { left, op, right } => {
                builder.push_sql("(");
                left.to_sql(builder);
                builder.push_sql(match op {
                    BinaryOp::Eq => " = ",
                    BinaryOp::Ne => " <> ",
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
        }
    }
}
