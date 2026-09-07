//! Typed relation paths and their query-local join planning.

use std::marker::PhantomData;
use std::ops::{Add, BitAnd, BitOr, BitXor, Deref, Div, Mul, Not, Shl, Shr, Sub};

use crate::expr::{
    BinaryOp, BitwiseOperand, ComparisonValue, ExprComparisonMarker, ExprOperand, NullableExprComparisonMarker, ValueComparisonMarker,
    ValueComparisonOutput,
};
use crate::query::{IntoOrderExpr, OrderExpr};
use crate::rel::Relation;
use crate::{Column, Expr, ExprNode, IntoExpr, Table, Value};

#[derive(Debug, PartialEq, Eq)]
pub struct RelationPath {
    pub previous: Option<&'static RelationPath>,
    pub relation: Relation,
}

impl RelationPath {
    pub fn steps(&self) -> Vec<Relation> {
        let mut steps = self.previous.map_or_else(Vec::new, Self::steps);
        steps.push(self.relation);
        steps
    }
}

pub trait PathKey: Send + Sync + 'static {
    const PATH: Option<&'static RelationPath>;
}

impl PathKey for () {
    const PATH: Option<&'static RelationPath> = None;
}

pub struct Next<P, K>(PhantomData<(P, K)>);
impl<P: PathKey, K: PathKey> PathKey for Next<P, K> {
    const PATH: Option<&'static RelationPath> = Some(&RelationPath {
        previous: P::PATH,
        relation: match K::PATH {
            Some(path) => path.relation,
            None => panic!("missing relation path"),
        },
    });
}

pub trait RelationFields<P: PathKey> {
    type Fields: 'static;
    const FIELDS: &'static Self::Fields;
}

pub struct Related<P, M>(PhantomData<(P, M)>);
impl<P, M> Related<P, M> {
    pub const NEW: Self = Self(PhantomData);
}
impl<P, M> Copy for Related<P, M> {}
impl<P, M> Clone for Related<P, M> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<P: PathKey, M: RelationFields<P>> Deref for Related<P, M> {
    type Target = M::Fields;
    fn deref(&self) -> &Self::Target {
        M::FIELDS
    }
}

// Keep mutation APIs accepting ordinary Column values. Deref reuses the column
// methods, while trait implementations preserve paths in functions and operators.
pub struct RelatedColumn<P, T>(Column<P, T>);
impl<P, T> Copy for RelatedColumn<P, T> {}
impl<P, T> Clone for RelatedColumn<P, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<P: PathKey, T> RelatedColumn<P, T> {
    pub const fn new(table: Table, name: &'static str) -> Self {
        Self(Column::new(table, name).with_path(P::PATH))
    }
}
impl<P, T> Deref for RelatedColumn<P, T> {
    type Target = Column<P, T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<P, T> IntoExpr<T> for RelatedColumn<P, T> {
    fn into_expr(self) -> Expr<T> {
        self.0.into_expr()
    }
}
impl<P, T> ExprOperand for RelatedColumn<P, T> {
    type Value = T;
    fn into_operand_expr(self) -> Expr<T> {
        self.into_expr()
    }
}
impl<P, T> BitwiseOperand for RelatedColumn<P, T> {
    type Value = T;
    fn into_bitwise_expr(self) -> Expr<T> {
        self.into_expr()
    }
}
impl<P, T> IntoOrderExpr for RelatedColumn<P, T> {
    fn into_order_expr(self) -> OrderExpr {
        self.0.into_order_expr()
    }
}
impl<P, T: ValueComparisonOutput> ComparisonValue<T, ExprComparisonMarker> for RelatedColumn<P, T> {
    type Output = T::Output;
    fn into_comparison_expr(self) -> Expr<T> {
        self.into_expr()
    }
}
impl<P, T> ComparisonValue<Option<T>, ExprComparisonMarker> for RelatedColumn<P, T> {
    type Output = Option<bool>;
    fn into_comparison_expr(self) -> Expr<Option<T>> {
        Expr::new(self.into_expr().node)
    }
}
impl<P, T> ComparisonValue<T, NullableExprComparisonMarker> for RelatedColumn<P, Option<T>> {
    type Output = Option<bool>;
    fn into_comparison_expr(self) -> Expr<T> {
        Expr::new(self.into_expr().node)
    }
}
/// Comparison operands for relation columns retain their Rust value types.
/// The general ordered-comparison API also accepts untyped SQL literals; paths
/// deliberately require ColumnValue<T> for literals, just like Column::eq.
pub trait PathComparisonValue<T, Marker> {
    type Output;
    fn into_path_expr(self) -> Expr<T>;
}
impl<T: ValueComparisonOutput, V: crate::ColumnValue<T>> PathComparisonValue<T, ValueComparisonMarker> for V {
    type Output = T::Output;
    fn into_path_expr(self) -> Expr<T> {
        Expr::new(ExprNode::Value(self.into_value().unwrap_or(Value::Null)))
    }
}
impl<T, V: ComparisonValue<T, ExprComparisonMarker>> PathComparisonValue<T, ExprComparisonMarker> for V {
    type Output = V::Output;
    fn into_path_expr(self) -> Expr<T> {
        self.into_comparison_expr()
    }
}
impl<T, V: ComparisonValue<T, NullableExprComparisonMarker>> PathComparisonValue<T, NullableExprComparisonMarker> for V {
    type Output = V::Output;
    fn into_path_expr(self) -> Expr<T> {
        self.into_comparison_expr()
    }
}
impl<P, T> RelatedColumn<P, T> {
    pub fn eq<V, Marker>(self, value: V) -> Expr<V::Output>
    where
        V: PathComparisonValue<T, Marker>,
    {
        self.compare(value, BinaryOp::Eq)
    }
    pub fn ne<V, Marker>(self, value: V) -> Expr<V::Output>
    where
        V: PathComparisonValue<T, Marker>,
    {
        self.compare(value, BinaryOp::Ne)
    }
    pub fn lt<V, Marker>(self, value: V) -> Expr<V::Output>
    where
        V: PathComparisonValue<T, Marker>,
    {
        self.compare(value, BinaryOp::Lt)
    }
    pub fn le<V, Marker>(self, value: V) -> Expr<V::Output>
    where
        V: PathComparisonValue<T, Marker>,
    {
        self.compare(value, BinaryOp::Le)
    }
    pub fn gt<V, Marker>(self, value: V) -> Expr<V::Output>
    where
        V: PathComparisonValue<T, Marker>,
    {
        self.compare(value, BinaryOp::Gt)
    }
    pub fn ge<V, Marker>(self, value: V) -> Expr<V::Output>
    where
        V: PathComparisonValue<T, Marker>,
    {
        self.compare(value, BinaryOp::Ge)
    }
    fn compare<V, Marker>(self, value: V, op: BinaryOp) -> Expr<V::Output>
    where
        V: PathComparisonValue<T, Marker>,
    {
        let right = value.into_path_expr().node;
        let left = Box::new(self.into_expr().node);
        if matches!(right, ExprNode::Value(Value::Null)) && matches!(op, BinaryOp::Eq | BinaryOp::Ne) {
            Expr::new(ExprNode::IsNull {
                expr: left,
                negated: matches!(op, BinaryOp::Ne),
            })
        } else {
            Expr::new(ExprNode::Binary {
                left,
                op,
                right: Box::new(right),
            })
        }
    }
}

macro_rules! forward_operator {
    ($($trait:ident::$method:ident),* $(,)?) => {$(
        impl<P, T, R> $trait<R> for RelatedColumn<P, T>
        where Column<P, T>: $trait<R> {
            type Output = <Column<P, T> as $trait<R>>::Output;
            fn $method(self, rhs: R) -> Self::Output { self.0.$method(rhs) }
        }
    )*};
}
forward_operator!(
    Add::add,
    Sub::sub,
    Mul::mul,
    Div::div,
    BitAnd::bitand,
    BitOr::bitor,
    BitXor::bitxor,
    Shl::shl,
    Shr::shr
);
impl<P, T> Not for RelatedColumn<P, T>
where
    Column<P, T>: Not,
{
    type Output = <Column<P, T> as Not>::Output;
    fn not(self) -> Self::Output {
        self.0.not()
    }
}

pub fn column(column: crate::ColumnRef, path: &[Relation]) -> ExprNode {
    if path.is_empty() {
        ExprNode::Column(column)
    } else {
        ExprNode::RelatedColumn {
            column,
            path: path.to_vec(),
        }
    }
}

pub fn join_on(path: &[Relation]) -> Expr<bool> {
    let (relation, previous) = path.split_last().expect("nonempty relation path");
    let (target, source) = match relation.kind {
        crate::RelationKind::BelongsTo => (relation.parent_key, relation.child_key),
        crate::RelationKind::HasMany => (relation.child_key, relation.parent_key),
        crate::RelationKind::ManyToMany => unreachable!("bridge joins are expanded before planning"),
    };
    Expr::new(ExprNode::Binary {
        left: Box::new(column(target, path)),
        op: BinaryOp::Eq,
        right: Box::new(column(source, previous)),
    })
}

impl ExprNode {
    pub(crate) fn visit_paths(&self, visit: &mut impl FnMut(&[Relation])) {
        match self {
            Self::Column(col) => {
                if let Some(path) = col.path {
                    visit(&path.steps());
                }
            }
            Self::RelatedColumn { path, .. } => visit(path),
            Self::Row { values } | Self::Func { args: values, .. } => {
                for value in values {
                    value.visit_paths(visit);
                }
            }
            Self::Trim { expr, characters, .. } => {
                expr.visit_paths(visit);
                if let Some(chars) = characters {
                    chars.visit_paths(visit);
                }
            }
            Self::AggregateFilter {
                aggregate: left,
                predicate: right,
            }
            | Self::VectorBinary { left, right, .. }
            | Self::Binary { left, right, .. }
            | Self::Bool { left, right, .. } => {
                left.visit_paths(visit);
                right.visit_paths(visit);
            }
            Self::Normalize { expr, .. }
            | Self::MakeInterval { value: expr, .. }
            | Self::Cast { expr, .. }
            | Self::Unary { expr, .. }
            | Self::In { expr, .. }
            | Self::RowIn { expr, .. }
            | Self::IsNull { expr, .. }
            | Self::Like { expr, .. } => expr.visit_paths(visit),
            // Subqueries discover their own paths when compiled in their enclosing scope.
            Self::Value(_) | Self::Exists { .. } => {}
        }
    }
}

enum PlannedJoin {
    Declared(crate::Join),
    Related {
        path: Vec<Relation>,
        alias: String,
        kind: crate::JoinKind,
    },
}

pub(crate) struct JoinPlan {
    joins: Vec<PlannedJoin>,
    reserved: Vec<String>,
}

impl JoinPlan {
    pub(crate) fn new(base: Table, declared: &[crate::Join], extra: &[crate::Join], outer_qualifiers: &[String]) -> Self {
        let mut plan = Self {
            joins: Vec::new(),
            reserved: std::iter::once(base.qualifier().to_owned())
                .chain(declared.iter().chain(extra).map(|join| join.table.qualifier().to_owned()))
                .chain(outer_qualifiers.iter().cloned())
                .collect(),
        };
        for (joins, explicit) in [(declared, true), (extra, false)] {
            for join in joins {
                if let ExprNode::Binary {
                    left, op: BinaryOp::Eq, ..
                } = &join.on.node
                {
                    if let ExprNode::RelatedColumn { path, column } = &**left {
                        if column.table == join.table {
                            plan.require(path, explicit.then_some(join.kind));
                            continue;
                        }
                    }
                }
                join.on.node.visit_paths(&mut |path| plan.require(path, None));
                plan.joins.push(PlannedJoin::Declared(join.clone()));
            }
        }
        plan
    }

    pub(crate) fn discover(&mut self, expr: &ExprNode) {
        expr.visit_paths(&mut |path| self.require(path, None));
    }

    fn require(&mut self, path: &[Relation], explicit: Option<crate::JoinKind>) {
        if path.is_empty() {
            return;
        }
        if let Some(PlannedJoin::Related { kind, .. }) = self
            .joins
            .iter_mut()
            .find(|join| matches!(join, PlannedJoin::Related { path: existing, .. } if existing == path))
        {
            if let Some(declared) = explicit {
                *kind = declared;
            }
            return;
        }
        self.require(&path[..path.len() - 1], None);
        let mut index = self.joins.len();
        let alias = loop {
            let alias = format!("__dbkit_r{index}");
            if !self.reserved.contains(&alias) {
                break alias;
            }
            index += 1;
        };
        self.reserved.push(alias.clone());
        self.joins.push(PlannedJoin::Related {
            path: path.to_vec(),
            alias,
            kind: explicit.unwrap_or(crate::JoinKind::Left),
        });
    }

    pub(crate) fn aliases(&self) -> Vec<(Vec<Relation>, String)> {
        self.joins
            .iter()
            .filter_map(|join| match join {
                PlannedJoin::Related { path, alias, .. } => Some((path.clone(), alias.clone())),
                PlannedJoin::Declared(_) => None,
            })
            .collect()
    }

    pub(crate) fn declared_tables(&self) -> Vec<Table> {
        self.joins
            .iter()
            .filter_map(|join| match join {
                PlannedJoin::Declared(join) => Some(join.table),
                PlannedJoin::Related { .. } => None,
            })
            .collect()
    }

    pub(crate) fn has_left_join(&self) -> bool {
        self.joins.iter().any(|join| {
            let kind = match join {
                PlannedJoin::Declared(join) => join.kind,
                PlannedJoin::Related { kind, .. } => *kind,
            };
            matches!(kind, crate::JoinKind::Left)
        })
    }

    pub(crate) fn write(&self, builder: &mut crate::compile::SqlBuilder) {
        use crate::compile::ToSql;
        for join in &self.joins {
            let (table, alias, kind, on) = match join {
                PlannedJoin::Declared(join) => (join.table, join.table.alias, join.kind, join.on.clone()),
                PlannedJoin::Related { path, alias, kind } => {
                    (path.last().unwrap().join_table(), Some(alias.as_str()), *kind, join_on(path))
                }
            };
            builder.push_sql(match kind {
                crate::JoinKind::Inner => " JOIN ",
                crate::JoinKind::Left => " LEFT JOIN ",
            });
            builder.push_sql(&table.qualified_name());
            if let Some(alias) = alias {
                builder.push_sql(" ");
                builder.push_sql(alias);
            }
            builder.push_sql(" ON ");
            on.node.to_sql(builder);
        }
    }
}
