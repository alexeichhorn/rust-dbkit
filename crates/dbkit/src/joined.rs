use crate::expr::ExprNode;
use crate::executor::{build_arguments, BoxFuture};
use crate::load::{ApplyLoad, Joined, LoadChain, NoLoad, SelectIn};
use crate::query::{Join, JoinKind, SelectItem};
use crate::rel::RelationInfo;
use crate::runtime::RunLoads;
use crate::schema::{ColumnRef, Table};
use crate::{Error, Executor, GetRelation, JoinedModel, ModelValue, Select, Value};
use sqlx::postgres::PgRow;

pub(crate) struct JoinedYes;
pub(crate) struct JoinedNo;

pub(crate) trait Or<Rhs> {
    type Output;
}

impl Or<JoinedNo> for JoinedNo {
    type Output = JoinedNo;
}

impl Or<JoinedYes> for JoinedNo {
    type Output = JoinedYes;
}

impl Or<JoinedNo> for JoinedYes {
    type Output = JoinedYes;
}

impl Or<JoinedYes> for JoinedYes {
    type Output = JoinedYes;
}

pub(crate) trait JoinedFlag {
    type Flag;
}

impl JoinedFlag for NoLoad {
    type Flag = JoinedNo;
}

impl<R, Nested> JoinedFlag for SelectIn<R, Nested> {
    type Flag = JoinedNo;
}

impl<R, Nested> JoinedFlag for Joined<R, Nested> {
    type Flag = JoinedYes;
}

impl<Prev, L> JoinedFlag for LoadChain<Prev, L>
where
    Prev: JoinedFlag,
    L: JoinedFlag,
    Prev::Flag: Or<L::Flag>,
{
    type Flag = <Prev::Flag as Or<L::Flag>>::Output;
}

pub(crate) struct Ops<Flag, Out, Loads>(std::marker::PhantomData<(Flag, Out, Loads)>);

pub(crate) trait JoinOps {
    type Out;
    type Loads;

    fn all<'e, E, Lock, DistinctState>(
        select: Select<Self::Out, Self::Loads, Lock, DistinctState>,
        ex: &'e E,
    ) -> BoxFuture<'e, Result<Vec<Self::Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Self::Out: 'e,
        Self::Loads: 'e;

    fn one<'e, E, Lock, DistinctState>(
        select: Select<Self::Out, Self::Loads, Lock, DistinctState>,
        ex: &'e E,
    ) -> BoxFuture<'e, Result<Option<Self::Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Self::Out: 'e,
        Self::Loads: 'e;
}

impl<Out, Loads> JoinOps for Ops<JoinedNo, Out, Loads>
where
    Loads: RunLoads<Out> + Send + Sync,
    Out: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin,
{
    type Out = Out;
    type Loads = Loads;

    fn all<'e, E, Lock, DistinctState>(
        select: Select<Out, Loads, Lock, DistinctState>,
        ex: &'e E,
    ) -> BoxFuture<'e, Result<Vec<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Out: 'e,
        Loads: 'e,
    {
        let (compiled, loads) = select.into_parts();
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            let mut rows = ex.fetch_all::<Out>(&compiled.sql, args).await?;
            loads.run(ex, &mut rows).await?;
            Ok(rows)
        })
    }

    fn one<'e, E, Lock, DistinctState>(
        select: Select<Out, Loads, Lock, DistinctState>,
        ex: &'e E,
    ) -> BoxFuture<'e, Result<Option<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Out: 'e,
        Loads: 'e,
    {
        let (compiled, loads) = select.limit(1).into_parts();
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            let row = ex.fetch_optional::<Out>(&compiled.sql, args).await?;
            let Some(value) = row else {
                return Ok(None);
            };
            let mut rows = vec![value];
            loads.run(ex, &mut rows).await?;
            Ok(rows.pop())
        })
    }
}

impl<Out, Loads> JoinOps for Ops<JoinedYes, Out, Loads>
where
    Loads: BuildJoinPlan<Out> + RunLoads<Out> + Send + Sync,
    Out: JoinedModel + ModelValue + for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin,
{
    type Out = Out;
    type Loads = Loads;

    fn all<'e, E, Lock, DistinctState>(
        select: Select<Out, Loads, Lock, DistinctState>,
        ex: &'e E,
    ) -> BoxFuture<'e, Result<Vec<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Out: 'e,
        Loads: 'e,
    {
        let (select, loads) = select.into_parts_with_loads();
        if select.columns_ref().is_some() {
            return Box::pin(async {
                Err(Error::Decode(
                    "joined eager loading requires base model selection".to_string(),
                ))
            });
        }
        let mut ctx = JoinContext::new(select.joins());
        let collectors = loads.build_collectors(&mut ctx);
        let compiled = select.compile_with_extra(&ctx.extra_columns, &ctx.extra_joins);

        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            let rows = ex.fetch_rows(&compiled.sql, args).await?;
            let mut out = decode_joined::<Out>(rows, &collectors)?;
            loads.run(ex, &mut out).await?;
            Ok(out)
        })
    }

    fn one<'e, E, Lock, DistinctState>(
        select: Select<Out, Loads, Lock, DistinctState>,
        ex: &'e E,
    ) -> BoxFuture<'e, Result<Option<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Out: 'e,
        Loads: 'e,
    {
        let (select, loads) = select.limit(1).into_parts_with_loads();
        if select.columns_ref().is_some() {
            return Box::pin(async {
                Err(Error::Decode(
                    "joined eager loading requires base model selection".to_string(),
                ))
            });
        }
        let mut ctx = JoinContext::new(select.joins());
        let collectors = loads.build_collectors(&mut ctx);
        let compiled = select.compile_with_extra(&ctx.extra_columns, &ctx.extra_joins);

        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            let rows = ex.fetch_rows(&compiled.sql, args).await?;
            let mut out = decode_joined::<Out>(rows, &collectors)?;
            if out.is_empty() {
                return Ok(None);
            }
            loads.run(ex, &mut out).await?;
            Ok(out.pop())
        })
    }
}

pub(crate) struct JoinContext {
    pub extra_columns: Vec<SelectItem>,
    pub extra_joins: Vec<Join>,
    joined_tables: Vec<Table>,
    next_alias: usize,
}

impl JoinContext {
    pub fn new(existing_joins: &[Join]) -> Self {
        let joined_tables = existing_joins.iter().map(|join| join.table).collect();
        Self {
            extra_columns: Vec::new(),
            extra_joins: Vec::new(),
            joined_tables,
            next_alias: 0,
        }
    }

    fn next_prefix(&mut self) -> String {
        let prefix = format!("__dbkit_j{}__", self.next_alias);
        self.next_alias += 1;
        prefix
    }

    fn ensure_join(&mut self, join: Join) {
        if self.joined_tables.iter().any(|table| *table == join.table) {
            return;
        }
        self.joined_tables.push(join.table);
        self.extra_joins.push(join);
    }

    fn add_columns<T: JoinedModel>(&mut self, prefix: &str) {
        for column in T::joined_columns() {
            let alias = format!("{}{}", prefix, column.name);
            self.extra_columns.push(SelectItem {
                expr: ExprNode::Column(*column),
                alias: Some(alias),
            });
        }
    }
}

pub(crate) trait JoinCollector<Parent>: Send {
    fn collect(&self, row: &PgRow, parent: &mut Parent) -> Result<(), Error>;
}

pub(crate) trait BuildJoinPlan<Out> {
    fn build_collectors(&self, ctx: &mut JoinContext) -> Vec<Box<dyn JoinCollector<Out>>>;
}

impl<Out> BuildJoinPlan<Out> for NoLoad {
    fn build_collectors(&self, _ctx: &mut JoinContext) -> Vec<Box<dyn JoinCollector<Out>>> {
        Vec::new()
    }
}

impl<Out, Prev, L> BuildJoinPlan<Out> for LoadChain<Prev, L>
where
    Prev: BuildJoinPlan<Out>,
    L: BuildJoinPlan<Out>,
{
    fn build_collectors(&self, ctx: &mut JoinContext) -> Vec<Box<dyn JoinCollector<Out>>> {
        let mut collectors = self.prev.build_collectors(ctx);
        collectors.extend(self.load.build_collectors(ctx));
        collectors
    }
}

impl<Out, R, Nested> BuildJoinPlan<Out> for SelectIn<R, Nested> {
    fn build_collectors(&self, _ctx: &mut JoinContext) -> Vec<Box<dyn JoinCollector<Out>>> {
        Vec::new()
    }
}

struct HasManyCollector<Rel, ChildOut> {
    rel: Rel,
    prefix: String,
    nested: Vec<Box<dyn JoinCollector<ChildOut>>>,
}

impl<ParentOut, Rel, ChildOut> JoinCollector<ParentOut> for HasManyCollector<Rel, ChildOut>
where
    ParentOut: GetRelation<Rel, Vec<ChildOut>>,
    Rel: Clone + Send,
    ChildOut: JoinedModel + ModelValue,
{
    fn collect(&self, row: &PgRow, parent: &mut ParentOut) -> Result<(), Error> {
        if !ChildOut::joined_row_has_pk(row, &self.prefix)? {
            return Ok(());
        }

        let child = ChildOut::joined_from_row_prefixed(row, &self.prefix)?;
        let child_key = model_key(&child)?;

        let Some(children) = parent.get_relation_mut(self.rel.clone()) else {
            return Err(Error::RelationMismatch);
        };

        if let Some(index) = find_child_index(children, &child_key)? {
            let existing = &mut children[index];
            for nested in &self.nested {
                nested.collect(row, existing)?;
            }
            return Ok(());
        }

        let mut child = child;
        for nested in &self.nested {
            nested.collect(row, &mut child)?;
        }
        children.push(child);
        Ok(())
    }
}

struct BelongsToCollector<Rel, ParentOut> {
    rel: Rel,
    prefix: String,
    nested: Vec<Box<dyn JoinCollector<ParentOut>>>,
}

impl<ChildOut, Rel, ParentOut> JoinCollector<ChildOut> for BelongsToCollector<Rel, ParentOut>
where
    ChildOut: GetRelation<Rel, Option<ParentOut>>,
    Rel: Clone + Send,
    ParentOut: JoinedModel + ModelValue,
{
    fn collect(&self, row: &PgRow, child: &mut ChildOut) -> Result<(), Error> {
        if !ParentOut::joined_row_has_pk(row, &self.prefix)? {
            return Ok(());
        }

        let Some(parent_slot) = child.get_relation_mut(self.rel.clone()) else {
            return Err(Error::RelationMismatch);
        };

        if let Some(existing) = parent_slot.as_mut() {
            for nested in &self.nested {
                nested.collect(row, existing)?;
            }
            return Ok(());
        }

        let mut parent = ParentOut::joined_from_row_prefixed(row, &self.prefix)?;
        for nested in &self.nested {
            nested.collect(row, &mut parent)?;
        }
        *parent_slot = Some(parent);
        Ok(())
    }
}

struct ManyToManyCollector<Rel, ChildOut> {
    rel: Rel,
    prefix: String,
    nested: Vec<Box<dyn JoinCollector<ChildOut>>>,
}

impl<ParentOut, Rel, ChildOut> JoinCollector<ParentOut> for ManyToManyCollector<Rel, ChildOut>
where
    ParentOut: GetRelation<Rel, Vec<ChildOut>>,
    Rel: Clone + Send,
    ChildOut: JoinedModel + ModelValue,
{
    fn collect(&self, row: &PgRow, parent: &mut ParentOut) -> Result<(), Error> {
        if !ChildOut::joined_row_has_pk(row, &self.prefix)? {
            return Ok(());
        }

        let child = ChildOut::joined_from_row_prefixed(row, &self.prefix)?;
        let child_key = model_key(&child)?;

        let Some(children) = parent.get_relation_mut(self.rel.clone()) else {
            return Err(Error::RelationMismatch);
        };

        if let Some(index) = find_child_index(children, &child_key)? {
            let existing = &mut children[index];
            for nested in &self.nested {
                nested.collect(row, existing)?;
            }
            return Ok(());
        }

        let mut child = child;
        for nested in &self.nested {
            nested.collect(row, &mut child)?;
        }
        children.push(child);
        Ok(())
    }
}

impl<Parent, Child, Nested, ParentOut> BuildJoinPlan<ParentOut> for Joined<crate::rel::HasMany<Parent, Child>, Nested>
where
    Nested: ApplyLoad<Child> + BuildJoinPlan<<Nested as ApplyLoad<Child>>::Out2>,
    <Nested as ApplyLoad<Child>>::Out2: JoinedModel + 'static,
    ParentOut: GetRelation<
        crate::rel::HasMany<Parent, Child>,
        Vec<<Nested as ApplyLoad<Child>>::Out2>,
    > + 'static,
    Parent: Clone + Send + 'static,
    Child: Clone + Send + 'static,
{
    fn build_collectors(&self, ctx: &mut JoinContext) -> Vec<Box<dyn JoinCollector<ParentOut>>> {
        let relation = self.rel.relation();
        for (table, on) in relation.join_steps() {
            ctx.ensure_join(Join {
                table,
                on,
                kind: JoinKind::Left,
            });
        }

        let prefix = ctx.next_prefix();
        ctx.add_columns::<<Nested as ApplyLoad<Child>>::Out2>(&prefix);

        let nested = <Nested as BuildJoinPlan<<Nested as ApplyLoad<Child>>::Out2>>::build_collectors(
            &self.nested,
            ctx,
        );

        let collector = HasManyCollector {
            rel: self.rel.clone(),
            prefix,
            nested,
        };
        vec![Box::new(collector)]
    }
}

impl<Child, Parent, Nested, ChildOut> BuildJoinPlan<ChildOut> for Joined<crate::rel::BelongsTo<Child, Parent>, Nested>
where
    Nested: ApplyLoad<Parent> + BuildJoinPlan<<Nested as ApplyLoad<Parent>>::Out2>,
    <Nested as ApplyLoad<Parent>>::Out2: JoinedModel + 'static,
    ChildOut: GetRelation<
        crate::rel::BelongsTo<Child, Parent>,
        Option<<Nested as ApplyLoad<Parent>>::Out2>,
    > + 'static,
    Child: Clone + Send + 'static,
    Parent: Clone + Send + 'static,
{
    fn build_collectors(&self, ctx: &mut JoinContext) -> Vec<Box<dyn JoinCollector<ChildOut>>> {
        let relation = self.rel.relation();
        for (table, on) in relation.join_steps() {
            ctx.ensure_join(Join {
                table,
                on,
                kind: JoinKind::Left,
            });
        }

        let prefix = ctx.next_prefix();
        ctx.add_columns::<<Nested as ApplyLoad<Parent>>::Out2>(&prefix);

        let nested = <Nested as BuildJoinPlan<<Nested as ApplyLoad<Parent>>::Out2>>::build_collectors(
            &self.nested,
            ctx,
        );

        let collector = BelongsToCollector {
            rel: self.rel.clone(),
            prefix,
            nested,
        };
        vec![Box::new(collector)]
    }
}

impl<Parent, Child, Through, Nested, ParentOut> BuildJoinPlan<ParentOut>
    for Joined<crate::rel::ManyToMany<Parent, Child, Through>, Nested>
where
    Nested: ApplyLoad<Child> + BuildJoinPlan<<Nested as ApplyLoad<Child>>::Out2>,
    <Nested as ApplyLoad<Child>>::Out2: JoinedModel + 'static,
    ParentOut: GetRelation<
        crate::rel::ManyToMany<Parent, Child, Through>,
        Vec<<Nested as ApplyLoad<Child>>::Out2>,
    > + 'static,
    Parent: Clone + Send + 'static,
    Child: Clone + Send + 'static,
    Through: Clone + Send + 'static,
{
    fn build_collectors(&self, ctx: &mut JoinContext) -> Vec<Box<dyn JoinCollector<ParentOut>>> {
        let relation = self.rel.relation();
        for (table, on) in relation.join_steps() {
            ctx.ensure_join(Join {
                table,
                on,
                kind: JoinKind::Left,
            });
        }

        let prefix = ctx.next_prefix();
        ctx.add_columns::<<Nested as ApplyLoad<Child>>::Out2>(&prefix);

        let nested = <Nested as BuildJoinPlan<<Nested as ApplyLoad<Child>>::Out2>>::build_collectors(
            &self.nested,
            ctx,
        );

        let collector = ManyToManyCollector {
            rel: self.rel.clone(),
            prefix,
            nested,
        };
        vec![Box::new(collector)]
    }
}

pub(crate) fn decode_joined<Out>(
    rows: Vec<PgRow>,
    collectors: &[Box<dyn JoinCollector<Out>>],
) -> Result<Vec<Out>, Error>
where
    Out: JoinedModel + ModelValue + for<'r> sqlx::FromRow<'r, PgRow>,
{
    let mut results: Vec<Out> = Vec::new();
    let mut keys: Vec<Vec<Value>> = Vec::new();

    for row in rows {
        let candidate: Out = Out::from_row(&row)?;
        let key = model_key(&candidate)?;

        let index = keys.iter().position(|existing| *existing == key);
        let target = if let Some(index) = index {
            &mut results[index]
        } else {
            results.push(candidate);
            keys.push(key);
            results.last_mut().expect("just pushed")
        };

        for collector in collectors {
            collector.collect(&row, target)?;
        }
    }

    Ok(results)
}

fn key_columns<M: JoinedModel>() -> &'static [ColumnRef] {
    let keys = M::joined_primary_keys();
    if keys.is_empty() {
        M::joined_columns()
    } else {
        keys
    }
}

fn model_key<M: JoinedModel + ModelValue>(model: &M) -> Result<Vec<Value>, Error> {
    let mut values = Vec::new();
    for column in key_columns::<M>() {
        let value = model.column_value(*column).ok_or(Error::RelationMismatch)?;
        values.push(value);
    }
    Ok(values)
}

fn find_child_index<ChildOut: JoinedModel + ModelValue>(
    children: &[ChildOut],
    key: &[Value],
) -> Result<Option<usize>, Error> {
    for (idx, child) in children.iter().enumerate() {
        if model_key(child)? == key {
            return Ok(Some(idx));
        }
    }
    Ok(None)
}
