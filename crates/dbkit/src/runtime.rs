use crate::executor::BoxFuture;
use crate::{
    Executor, ModelValue, Select, SetRelation, Value, SelectExt,
};
use crate::load::{LoadChain, NoLoad};
use crate::rel::RelationInfo;
use crate::{Error, Expr, ExprNode};

pub trait RunLoads<Out> {
    fn run<'e, E>(&self, ex: E, rows: &mut [Out]) -> BoxFuture<'e, Result<(), Error>>
    where
        E: Executor + 'e;
}

impl<Out> RunLoads<Out> for NoLoad {
    fn run<'e, E>(&self, _ex: E, _rows: &mut [Out]) -> BoxFuture<'e, Result<(), Error>>
    where
        E: Executor + 'e,
    {
        Box::pin(async { Ok(()) })
    }
}

impl<Out, Prev, L> RunLoads<Out> for LoadChain<Prev, L>
where
    Prev: RunLoads<Out>,
    L: RunLoad<Out>,
{
    fn run<'e, E>(&self, ex: E, rows: &mut [Out]) -> BoxFuture<'e, Result<(), Error>>
    where
        E: Executor + 'e,
    {
        Box::pin(async move {
            self.prev.run(ex, rows).await?;
            self.load.run(ex, rows).await
        })
    }
}

pub trait RunLoad<Out> {
    fn run<'e, E>(&self, ex: E, rows: &mut [Out]) -> BoxFuture<'e, Result<(), Error>>
    where
        E: Executor + 'e;
}

pub fn load_selectin_has_many<'e, E, Out, Rel, ChildOut, Nested>(
    ex: E,
    rows: &mut [Out],
    rel: Rel,
    nested: &Nested,
) -> BoxFuture<'e, Result<(), Error>>
where
    E: Executor + 'e,
    Rel: RelationInfo,
    Out: ModelValue + SetRelation<Rel, Vec<ChildOut>>,
    ChildOut: ModelValue + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    Nested: RunLoads<ChildOut>,
{
    Box::pin(async move {
        if rows.is_empty() {
            return Ok(());
        }

        let relation = rel.relation();
        let mut keys: Vec<Value> = Vec::new();
        for row in rows.iter() {
            if let Some(value) = row.column_value(relation.parent_key) {
                if value == Value::Null {
                    continue;
                }
                if !keys.iter().any(|existing| existing == &value) {
                    keys.push(value);
                }
            }
        }

        if keys.is_empty() {
            return Ok(());
        }

        let filter = Expr::new(ExprNode::In {
            expr: Box::new(ExprNode::Column(relation.child_key)),
            values: keys.clone(),
        });

        let query: Select<ChildOut> = Select::new(relation.child).filter(filter);
        let mut children = query.all(ex).await?;
        nested.run(ex, &mut children).await?;

        let mut groups: Vec<(Value, Vec<ChildOut>)> = Vec::new();
        for child in children {
            let Some(key) = child.column_value(relation.child_key) else {
                return Err(Error::RelationMismatch);
            };
            if key == Value::Null {
                continue;
            }
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, items)) => items.push(child),
                None => groups.push((key, vec![child])),
            }
        }

        for row in rows.iter_mut() {
            let Some(key) = row.column_value(relation.parent_key) else {
                return Err(Error::RelationMismatch);
            };
            if key == Value::Null {
                row.set_relation(rel, Vec::new())?;
                continue;
            }
            let mut value = Vec::new();
            if let Some((_, items)) = groups.iter_mut().find(|(k, _)| *k == key) {
                value = std::mem::take(items);
            }
            row.set_relation(rel, value)?;
        }

        Ok(())
    })
}

pub fn load_selectin_belongs_to<'e, E, Out, Rel, ParentOut, Nested>(
    ex: E,
    rows: &mut [Out],
    rel: Rel,
    nested: &Nested,
) -> BoxFuture<'e, Result<(), Error>>
where
    E: Executor + 'e,
    Rel: RelationInfo,
    Out: ModelValue + SetRelation<Rel, Option<ParentOut>>,
    ParentOut: ModelValue + Clone + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    Nested: RunLoads<ParentOut>,
{
    Box::pin(async move {
        if rows.is_empty() {
            return Ok(());
        }

        let relation = rel.relation();
        let mut keys: Vec<Value> = Vec::new();
        for row in rows.iter() {
            if let Some(value) = row.column_value(relation.child_key) {
                if value == Value::Null {
                    continue;
                }
                if !keys.iter().any(|existing| existing == &value) {
                    keys.push(value);
                }
            }
        }

        if keys.is_empty() {
            return Ok(());
        }

        let filter = Expr::new(ExprNode::In {
            expr: Box::new(ExprNode::Column(relation.parent_key)),
            values: keys.clone(),
        });

        let query: Select<ParentOut> = Select::new(relation.parent).filter(filter);
        let mut parents = query.all(ex).await?;
        nested.run(ex, &mut parents).await?;

        for row in rows.iter_mut() {
            let key = row.column_value(relation.child_key);
            let value = key.and_then(|key| {
                if key == Value::Null {
                    return None;
                }
                parents
                    .iter()
                    .find(|parent| parent.column_value(relation.parent_key) == Some(key.clone()))
                    .cloned()
            });
            row.set_relation(rel, value)?;
        }

        Ok(())
    })
}

pub fn load_joined_has_many<'e, E, Out, Rel, ChildOut, Nested>(
    ex: E,
    rows: &mut [Out],
    rel: Rel,
    nested: &Nested,
) -> BoxFuture<'e, Result<(), Error>>
where
    E: Executor + 'e,
    Rel: RelationInfo,
    Out: ModelValue + SetRelation<Rel, Vec<ChildOut>>,
    ChildOut: ModelValue + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    Nested: RunLoads<ChildOut>,
{
    Box::pin(async move { load_selectin_has_many(ex, rows, rel, nested).await })
}

pub fn load_joined_belongs_to<'e, E, Out, Rel, ParentOut, Nested>(
    ex: E,
    rows: &mut [Out],
    rel: Rel,
    nested: &Nested,
) -> BoxFuture<'e, Result<(), Error>>
where
    E: Executor + 'e,
    Rel: RelationInfo,
    Out: ModelValue + SetRelation<Rel, Option<ParentOut>>,
    ParentOut: ModelValue + Clone + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    Nested: RunLoads<ParentOut>,
{
    Box::pin(async move { load_selectin_belongs_to(ex, rows, rel, nested).await })
}
