use crate::executor::BoxFuture;
use crate::{
    Executor, GetRelation, ModelValue, Select, SetRelation, Value, SelectExt,
};
use crate::load::{LoadChain, NoLoad};
use crate::rel::{ManyToManyThrough, RelationInfo};
use crate::{Error, Expr, ExprNode};

pub trait RunLoads<Out> {
    fn run<'e, E>(&'e self, ex: &'e E, rows: &'e mut [Out]) -> BoxFuture<'e, Result<(), Error>>
    where
        E: Executor + Send + Sync + 'e,
        Out: Send + 'e;
}

impl<Out> RunLoads<Out> for NoLoad {
    fn run<'e, E>(&'e self, _ex: &'e E, _rows: &'e mut [Out]) -> BoxFuture<'e, Result<(), Error>>
    where
        E: Executor + Send + Sync + 'e,
        Out: Send + 'e,
    {
        Box::pin(async { Ok(()) })
    }
}

impl<Out, Prev, L> RunLoads<Out> for LoadChain<Prev, L>
where
    Prev: RunLoads<Out> + Sync,
    L: RunLoad<Out> + Sync,
{
    fn run<'e, E>(&'e self, ex: &'e E, rows: &'e mut [Out]) -> BoxFuture<'e, Result<(), Error>>
    where
        E: Executor + Send + Sync + 'e,
        Out: Send + 'e,
    {
        Box::pin(async move {
            self.prev.run(ex, rows).await?;
            self.load.run(ex, rows).await
        })
    }
}

pub trait RunLoad<Out> {
    fn run<'e, E>(&'e self, ex: &'e E, rows: &'e mut [Out]) -> BoxFuture<'e, Result<(), Error>>
    where
        E: Executor + Send + Sync + 'e,
        Out: Send + 'e;
}

pub fn load_selectin_has_many<'e, E, Out, Rel, ChildOut, Nested>(
    ex: &'e E,
    rows: &'e mut [Out],
    rel: Rel,
    nested: &'e Nested,
) -> BoxFuture<'e, Result<(), Error>>
where
    E: Executor + Send + Sync + 'e,
    Rel: RelationInfo + Clone + Send + 'e,
    Out: ModelValue + SetRelation<Rel, Vec<ChildOut>> + Send,
    ChildOut: ModelValue + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    Nested: RunLoads<ChildOut> + Sync,
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
            row.set_relation(rel.clone(), Vec::new())?;
            continue;
        }
            let mut value = Vec::new();
            if let Some((_, items)) = groups.iter_mut().find(|(k, _)| *k == key) {
                value = std::mem::take(items);
            }
            row.set_relation(rel.clone(), value)?;
        }

        Ok(())
    })
}

pub fn load_selectin_belongs_to<'e, E, Out, Rel, ParentOut, Nested>(
    ex: &'e E,
    rows: &'e mut [Out],
    rel: Rel,
    nested: &'e Nested,
) -> BoxFuture<'e, Result<(), Error>>
where
    E: Executor + Send + Sync + 'e,
    Rel: RelationInfo + Clone + Send + 'e,
    Out: ModelValue + SetRelation<Rel, Option<ParentOut>> + Send,
    ParentOut: ModelValue + Clone + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    Nested: RunLoads<ParentOut> + Sync,
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
            row.set_relation(rel.clone(), value)?;
        }

        Ok(())
    })
}

pub fn load_joined_has_many<'e, E, Out, Rel, ChildOut, Nested>(
    ex: &'e E,
    rows: &'e mut [Out],
    rel: Rel,
    nested: &'e Nested,
) -> BoxFuture<'e, Result<(), Error>>
where
    E: Executor + Send + Sync + 'e,
    Rel: RelationInfo + Clone + Send + 'e,
    Out: GetRelation<Rel, Vec<ChildOut>> + Send,
    ChildOut: ModelValue + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    Nested: RunLoads<ChildOut> + Sync,
{
    Box::pin(async move {
        for row in rows.iter_mut() {
            let Some(children) = row.get_relation_mut(rel.clone()) else {
                return Err(Error::RelationMismatch);
            };
            if !children.is_empty() {
                nested.run(ex, children.as_mut_slice()).await?;
            }
        }
        Ok(())
    })
}

pub fn load_selectin_many_to_many<'e, E, Out, Rel, Through, ChildOut, Nested>(
    ex: &'e E,
    rows: &'e mut [Out],
    rel: Rel,
    nested: &'e Nested,
) -> BoxFuture<'e, Result<(), Error>>
where
    E: Executor + Send + Sync + 'e,
    Rel: RelationInfo + ManyToManyThrough<Through = Through> + Clone + Send + 'e,
    Out: ModelValue + SetRelation<Rel, Vec<ChildOut>> + Send,
    Through: ModelValue + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    ChildOut: ModelValue + Clone + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    Nested: RunLoads<ChildOut> + Sync,
{
    Box::pin(async move {
        if rows.is_empty() {
            return Ok(());
        }

        let relation = rel.relation();
        let join_parent_key = relation
            .join_parent_key
            .ok_or(Error::RelationMismatch)?;
        let join_child_key = relation
            .join_child_key
            .ok_or(Error::RelationMismatch)?;
        let join_table = relation.join_table.ok_or(Error::RelationMismatch)?;

        let mut parent_keys: Vec<Value> = Vec::new();
        for row in rows.iter() {
            if let Some(value) = row.column_value(relation.parent_key) {
                if value == Value::Null {
                    continue;
                }
                if !parent_keys.iter().any(|existing| existing == &value) {
                    parent_keys.push(value);
                }
            }
        }

        if parent_keys.is_empty() {
            for row in rows.iter_mut() {
                row.set_relation(rel.clone(), Vec::new())?;
            }
            return Ok(());
        }

        let join_filter = Expr::new(ExprNode::In {
            expr: Box::new(ExprNode::Column(join_parent_key)),
            values: parent_keys.clone(),
        });
        let join_query: Select<Through> = Select::new(join_table).filter(join_filter);
        let join_rows = join_query.all(ex).await?;

        let mut parent_to_child: Vec<(Value, Vec<Value>)> = Vec::new();
        for row in join_rows {
            let Some(parent_key) = row.column_value(join_parent_key) else {
                return Err(Error::RelationMismatch);
            };
            let Some(child_key) = row.column_value(join_child_key) else {
                return Err(Error::RelationMismatch);
            };
            if parent_key == Value::Null || child_key == Value::Null {
                continue;
            }
            match parent_to_child
                .iter_mut()
                .find(|(key, _)| *key == parent_key)
            {
                Some((_, children)) => {
                    if !children.iter().any(|existing| existing == &child_key) {
                        children.push(child_key);
                    }
                }
                None => parent_to_child.push((parent_key, vec![child_key])),
            }
        }

        let mut child_keys: Vec<Value> = Vec::new();
        for (_, children) in parent_to_child.iter() {
            for child_key in children {
                if !child_keys.iter().any(|existing| existing == child_key) {
                    child_keys.push(child_key.clone());
                }
            }
        }

        if child_keys.is_empty() {
            for row in rows.iter_mut() {
                row.set_relation(rel.clone(), Vec::new())?;
            }
            return Ok(());
        }

        let child_filter = Expr::new(ExprNode::In {
            expr: Box::new(ExprNode::Column(relation.child_key)),
            values: child_keys.clone(),
        });
        let child_query: Select<ChildOut> = Select::new(relation.child).filter(child_filter);
        let mut children = child_query.all(ex).await?;
        nested.run(ex, &mut children).await?;

        for row in rows.iter_mut() {
            let Some(parent_key) = row.column_value(relation.parent_key) else {
                return Err(Error::RelationMismatch);
            };
            if parent_key == Value::Null {
                row.set_relation(rel.clone(), Vec::new())?;
                continue;
            }
            let mut value = Vec::new();
            if let Some((_, child_ids)) =
                parent_to_child.iter().find(|(key, _)| *key == parent_key)
            {
                for child_id in child_ids {
                    if let Some(child) = children
                        .iter()
                        .find(|child| child.column_value(relation.child_key) == Some(child_id.clone()))
                    {
                        value.push(child.clone());
                    }
                }
            }
            row.set_relation(rel.clone(), value)?;
        }

        Ok(())
    })
}

pub fn load_joined_belongs_to<'e, E, Out, Rel, ParentOut, Nested>(
    ex: &'e E,
    rows: &'e mut [Out],
    rel: Rel,
    nested: &'e Nested,
) -> BoxFuture<'e, Result<(), Error>>
where
    E: Executor + Send + Sync + 'e,
    Rel: RelationInfo + Clone + Send + 'e,
    Out: GetRelation<Rel, Option<ParentOut>> + Send,
    ParentOut: ModelValue + Clone + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    Nested: RunLoads<ParentOut> + Sync,
{
    Box::pin(async move {
        for row in rows.iter_mut() {
            let Some(parent) = row.get_relation_mut(rel.clone()) else {
                return Err(Error::RelationMismatch);
            };
            if let Some(parent) = parent.as_mut() {
                nested.run(ex, std::slice::from_mut(parent)).await?;
            }
        }
        Ok(())
    })
}

pub fn load_joined_many_to_many<'e, E, Out, Rel, Through, ChildOut, Nested>(
    ex: &'e E,
    rows: &'e mut [Out],
    rel: Rel,
    nested: &'e Nested,
) -> BoxFuture<'e, Result<(), Error>>
where
    E: Executor + Send + Sync + 'e,
    Rel: RelationInfo + ManyToManyThrough<Through = Through> + Clone + Send + 'e,
    Out: GetRelation<Rel, Vec<ChildOut>> + Send,
    Through: ModelValue + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    ChildOut: ModelValue + Clone + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    Nested: RunLoads<ChildOut> + Sync,
{
    Box::pin(async move {
        for row in rows.iter_mut() {
            let Some(children) = row.get_relation_mut(rel.clone()) else {
                return Err(Error::RelationMismatch);
            };
            if !children.is_empty() {
                nested.run(ex, children.as_mut_slice()).await?;
            }
        }
        Ok(())
    })
}
