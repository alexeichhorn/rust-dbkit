//@check-pass
use dbkit::model;

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "publication_state", rename_all = "snake_case")]
pub enum PublicationState {
    Draft,
    InReview,
    Published,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "review_outcome", rename_all = "snake_case")]
pub enum ReviewOutcome {
    Pending,
    Approved,
    Rejected,
}

#[model(table = "articles")]
pub struct Article {
    #[key]
    pub id: i64,
    pub slug: String,
    pub state: PublicationState,
    pub review: Option<ReviewOutcome>,
}

fn assert_from_row<T>()
where
    T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow>,
{
}

fn main() {
    assert_from_row::<ArticleModel>();

    let _query = Article::query()
        .filter(Article::state.eq(PublicationState::Draft))
        .filter(Article::state.ne(PublicationState::Archived))
        .filter(Article::state.in_([PublicationState::Draft, PublicationState::Published]))
        .filter(Article::review.eq(None::<ReviewOutcome>))
        .filter(Article::review.in_([Some(ReviewOutcome::Pending), Some(ReviewOutcome::Approved)]));

    let _insert = Article::insert(ArticleInsert {
        id: 1,
        slug: "intro-to-rust".to_string(),
        state: PublicationState::Draft,
        review: Some(ReviewOutcome::Pending),
    })
    .on_conflict_do_update(Article::slug, (Article::state, Article::review))
    .returning_all();

    let _update = Article::update()
        .set(Article::state, PublicationState::Published)
        .set(Article::review, Some(ReviewOutcome::Approved))
        .filter(Article::slug.eq("intro-to-rust"))
        .returning_all();

    let _clear_review = Article::update()
        .set(Article::review, None::<ReviewOutcome>)
        .filter(Article::id.eq(1_i64));

    let mut active = Article::new_active();
    active.id = 7_i64.into();
    active.slug = "status-active".to_string().into();
    active.state = PublicationState::InReview.into();
    active.review = None::<ReviewOutcome>.into();

    let _ = active;
}
