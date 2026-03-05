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
        }
    }
}
