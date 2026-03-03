//@check-pass
use dbkit::{model, Order};

#[model(table = "users")]
pub struct User {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub email: String,
}

fn main() {
    let _for_update_sql = User::query().for_update().debug_sql();

    let _skip_locked_sql = User::query()
        .filter(User::email.ilike("%@example.com"))
        .order_by(Order::asc(User::id))
        .limit(50)
        .for_update()
        .skip_locked()
        .debug_sql();

    let _nowait_sql = User::query()
        .order_by(Order::desc(User::id))
        .offset(10)
        .for_update()
        .nowait()
        .debug_sql();
}
