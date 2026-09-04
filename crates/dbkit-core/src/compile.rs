use crate::expr::{BinaryOp, BoolOp, ExprNode, IntervalField, TrimDirection, UnaryOp, Value, VectorBinaryOp};
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
            if let Some(end) = quoted_or_commented_region_end(bytes, idx) {
                idx = end;
            } else if bytes[idx] == b'$' {
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
                    // Cast suffixes remain in the source SQL and are copied on the next pass.
                    self.push_placeholder(value);
                    idx = end;
                    segment_start = end;
                    continue;
                }
                idx += 1;
            } else {
                idx += 1;
            }
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

fn quoted_or_commented_region_end(bytes: &[u8], start: usize) -> Option<usize> {
    match bytes[start] {
        b'\'' => Some(quoted_region_end(bytes, start, b'\'', is_escape_string_start(bytes, start))),
        b'"' => Some(quoted_region_end(bytes, start, b'"', false)),
        b'-' if bytes.get(start + 1) == Some(&b'-') => Some(
            bytes[start + 2..]
                .iter()
                .position(|byte| matches!(byte, b'\r' | b'\n'))
                .map_or(bytes.len(), |offset| start + offset + 3),
        ),
        b'/' if bytes.get(start + 1) == Some(&b'*') => Some(block_comment_end(bytes, start)),
        b'$' if start == 0 || !is_bind_ident_char(bytes[start - 1]) => {
            let delimiter_end = dollar_quote_delimiter_end(bytes, start)?;
            Some(dollar_quoted_region_end(bytes, start, delimiter_end))
        }
        _ => None,
    }
}

fn quoted_region_end(bytes: &[u8], start: usize, quote: u8, backslash_escapes: bool) -> usize {
    let mut idx = start + 1;
    while idx < bytes.len() {
        if backslash_escapes && bytes[idx] == b'\\' {
            idx = (idx + 2).min(bytes.len());
        } else if bytes[idx] == quote {
            if bytes.get(idx + 1) == Some(&quote) {
                idx += 2;
            } else {
                return idx + 1;
            }
        } else {
            idx += 1;
        }
    }
    bytes.len()
}

fn is_escape_string_start(bytes: &[u8], quote_idx: usize) -> bool {
    quote_idx > 0 && matches!(bytes[quote_idx - 1], b'e' | b'E') && (quote_idx == 1 || !is_bind_ident_char(bytes[quote_idx - 2]))
}

fn dollar_quote_delimiter_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut idx = start + 1;
    let first = *bytes.get(idx)?;
    if first == b'$' {
        return Some(idx + 1);
    }
    if !is_dollar_quote_tag_start(first) {
        return None;
    }

    idx += 1;
    while idx < bytes.len() && is_dollar_quote_tag_continue(bytes[idx]) {
        idx += 1;
    }

    (idx < bytes.len() && bytes[idx] == b'$').then_some(idx + 1)
}

fn dollar_quoted_region_end(bytes: &[u8], start: usize, delimiter_end: usize) -> usize {
    let delimiter = &bytes[start..delimiter_end];
    bytes[delimiter_end..]
        .windows(delimiter.len())
        .position(|candidate| candidate == delimiter)
        .map_or(bytes.len(), |offset| delimiter_end + offset + delimiter.len())
}

fn block_comment_end(bytes: &[u8], start: usize) -> usize {
    let mut idx = start + 2;
    let mut depth = 1;
    while idx < bytes.len() {
        if bytes[idx..].starts_with(b"/*") {
            depth += 1;
            idx += 2;
        } else if bytes[idx..].starts_with(b"*/") {
            depth -= 1;
            idx += 2;
            if depth == 0 {
                return idx;
            }
        } else {
            idx += 1;
        }
    }
    bytes.len()
}

fn is_dollar_quote_tag_start(byte: u8) -> bool {
    !byte.is_ascii() || byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_dollar_quote_tag_continue(byte: u8) -> bool {
    is_dollar_quote_tag_start(byte) || byte.is_ascii_digit()
}

pub trait ToSql {
    fn to_sql(&self, builder: &mut SqlBuilder);
}

impl ToSql for ExprNode {
    fn to_sql(&self, builder: &mut SqlBuilder) {
        match self {
            ExprNode::RawSql(sql) => builder.push_sql(sql),
            ExprNode::Column(col) => builder.push_column(*col),
            ExprNode::Value(value) => builder.push_value(value.clone()),
            ExprNode::Row { values } => {
                builder.push_sql("(");
                for (idx, value) in values.iter().enumerate() {
                    if idx > 0 {
                        builder.push_sql(", ");
                    }
                    value.to_sql(builder);
                }
                builder.push_sql(")");
            }
            ExprNode::Func { name, args } => {
                builder.push_sql(name);
                builder.push_sql("(");
                for (idx, arg) in args.iter().enumerate() {
                    if idx > 0 {
                        builder.push_sql(", ");
                    }
                    arg.to_sql(builder);
                }
                if (*name == "CONCAT" && args.is_empty()) || (*name == "CONCAT_WS" && args.len() == 1) {
                    if !args.is_empty() {
                        builder.push_sql(", ");
                    }
                    builder.push_sql("VARIADIC ARRAY[]::TEXT[]");
                }
                builder.push_sql(")");
            }
            ExprNode::Normalize { expr, form } => {
                builder.push_sql("NORMALIZE(");
                expr.to_sql(builder);
                builder.push_sql(", ");
                builder.push_sql(match form {
                    crate::func::NormalizationForm::Nfc => "NFC",
                    crate::func::NormalizationForm::Nfd => "NFD",
                    crate::func::NormalizationForm::Nfkc => "NFKC",
                    crate::func::NormalizationForm::Nfkd => "NFKD",
                });
                builder.push_sql(")");
            }
            ExprNode::Trim {
                direction,
                expr,
                characters,
            } => {
                builder.push_sql("TRIM(");
                builder.push_sql(match direction {
                    TrimDirection::Both => "BOTH",
                    TrimDirection::Leading => "LEADING",
                    TrimDirection::Trailing => "TRAILING",
                });
                if let Some(characters) = characters {
                    builder.push_sql(" ");
                    characters.to_sql(builder);
                }
                builder.push_sql(" FROM ");
                expr.to_sql(builder);
                builder.push_sql(")");
            }
            ExprNode::AggregateFilter { aggregate, predicate } => {
                aggregate.to_sql(builder);
                builder.push_sql(" FILTER (WHERE ");
                predicate.to_sql(builder);
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
                    BinaryOp::BitAnd => " & ",
                    BinaryOp::BitOr => " | ",
                    BinaryOp::BitXor => " # ",
                    BinaryOp::Shl => " << ",
                    BinaryOp::Shr => " >> ",
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
            ExprNode::Unary { op, expr } => match op {
                UnaryOp::Not => {
                    builder.push_sql("NOT (");
                    expr.to_sql(builder);
                    builder.push_sql(")");
                }
                UnaryOp::BitNot => {
                    builder.push_sql("(~");
                    expr.to_sql(builder);
                    builder.push_sql(")");
                }
            },
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
            ExprNode::RowIn { expr, rows } => {
                if rows.is_empty() {
                    builder.push_sql("(FALSE)");
                    return;
                }
                builder.push_sql("(");
                expr.to_sql(builder);
                builder.push_sql(" IN (");
                for (row_idx, row) in rows.iter().enumerate() {
                    if row_idx > 0 {
                        builder.push_sql(", ");
                    }
                    builder.push_sql("(");
                    for (value_idx, value) in row.iter().enumerate() {
                        if value_idx > 0 {
                            builder.push_sql(", ");
                        }
                        builder.push_value(value.clone());
                    }
                    builder.push_sql(")");
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
