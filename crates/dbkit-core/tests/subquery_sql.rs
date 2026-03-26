use dbkit_core::{expr::Value, func, Column, Order, Select, Table};

#[derive(Debug)]
struct Account;

#[derive(Debug)]
struct Invoice;

#[derive(Debug)]
struct Employee;

fn accounts_table() -> Table {
    Table::new("accounts")
}

fn account_id() -> Column<Account, i64> {
    Column::new(accounts_table(), "id")
}

fn account_region() -> Column<Account, String> {
    Column::new(accounts_table(), "region")
}

fn account_status() -> Column<Account, String> {
    Column::new(accounts_table(), "status")
}

fn invoices_table() -> Table {
    Table::new("invoices")
}

fn invoice_id() -> Column<Invoice, i64> {
    Column::new(invoices_table(), "id")
}

fn invoice_account_id() -> Column<Invoice, i64> {
    Column::new(invoices_table(), "account_id")
}

fn invoice_status() -> Column<Invoice, String> {
    Column::new(invoices_table(), "status")
}

fn employees_table() -> Table {
    Table::new("employees")
}

fn employee_id() -> Column<Employee, i64> {
    Column::new(employees_table(), "id")
}

fn employee_name() -> Column<Employee, String> {
    Column::new(employees_table(), "name")
}

#[test]
fn compiles_where_exists_with_correlated_subquery_and_binds() {
    let subquery: Select<Invoice> = Select::new(invoices_table())
        .select_only()
        .column(invoice_id())
        .filter(invoice_account_id().eq_col(account_id()))
        .filter(invoice_status().eq("open"));

    let query: Select<Account> = Select::new(accounts_table())
        .filter(account_region().eq("eu"))
        .where_exists(subquery);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT accounts.* FROM accounts WHERE (accounts.region = $1) AND EXISTS (SELECT invoices.id FROM invoices WHERE (invoices.account_id = accounts.id) AND (invoices.status = $2))"
    );
    assert_eq!(sql.binds, vec![Value::String("eu".to_string()), Value::String("open".to_string())]);
}

#[test]
fn compiles_where_not_exists_with_correlated_subquery_and_ordering() {
    let subquery: Select<Invoice> = Select::new(invoices_table())
        .select_only()
        .column(invoice_id())
        .filter(invoice_account_id().eq_col(account_id()))
        .filter(invoice_status().eq("void"));

    let query: Select<Account> = Select::new(accounts_table())
        .filter(account_status().eq("active"))
        .where_not_exists(subquery)
        .order_by(Order::asc(account_id()));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT accounts.* FROM accounts WHERE (accounts.status = $1) AND NOT (EXISTS (SELECT invoices.id FROM invoices WHERE (invoices.account_id = accounts.id) AND (invoices.status = $2))) ORDER BY accounts.id ASC"
    );
    assert_eq!(
        sql.binds,
        vec![Value::String("active".to_string()), Value::String("void".to_string())]
    );
}

#[test]
fn compiles_exists_expression_via_func_helper() {
    let subquery: Select<Invoice> = Select::new(invoices_table())
        .select_only()
        .column(invoice_id())
        .filter(invoice_account_id().eq_col(account_id()))
        .filter(invoice_status().eq("overdue"));

    let query: Select<Account> = Select::new(accounts_table())
        .filter(account_status().eq("active"))
        .filter(func::exists(subquery).not());

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT accounts.* FROM accounts WHERE (accounts.status = $1) AND NOT (EXISTS (SELECT invoices.id FROM invoices WHERE (invoices.account_id = accounts.id) AND (invoices.status = $2)))"
    );
    assert_eq!(
        sql.binds,
        vec![Value::String("active".to_string()), Value::String("overdue".to_string())]
    );
}

#[test]
fn compiles_where_exists_for_self_correlated_subquery_with_alias() {
    let reports_table = employees_table().with_alias("reports");
    let report_id: Column<Employee, i64> = Column::new(reports_table, "id");
    let report_manager_id: Column<Employee, i64> = Column::new(reports_table, "manager_id");

    let subquery: Select<Employee> = Select::new(reports_table)
        .select_only()
        .column(report_id)
        .filter(report_manager_id.eq_col(employee_id()));

    let query: Select<Employee> = Select::new(employees_table())
        .filter(employee_name().eq("Ava"))
        .where_exists(subquery);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT employees.* FROM employees WHERE (employees.name = $1) AND EXISTS (SELECT reports.id FROM employees reports WHERE (reports.manager_id = employees.id))"
    );
    assert_eq!(sql.binds, vec![Value::String("Ava".to_string())]);
}

#[test]
fn exists_subquery_does_not_treat_dollar_suffix_in_alias_as_bind_token() {
    let reports_table = employees_table().with_alias("reports$1");
    let report_id: Column<Employee, i64> = Column::new(reports_table, "id");
    let report_manager_id: Column<Employee, i64> = Column::new(reports_table, "manager_id");

    let subquery: Select<Employee> = Select::new(reports_table)
        .select_only()
        .column(report_id)
        .filter(report_manager_id.eq_col(employee_id()));

    let query: Select<Employee> = Select::new(employees_table()).where_exists(subquery);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT employees.* FROM employees WHERE EXISTS (SELECT reports$1.id FROM employees reports$1 WHERE (reports$1.manager_id = employees.id))"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn exists_subquery_preserves_utf8_alias_text() {
    let reports_table = employees_table().with_alias("caf\u{e9}");
    let report_id: Column<Employee, i64> = Column::new(reports_table, "id");
    let report_manager_id: Column<Employee, i64> = Column::new(reports_table, "manager_id");

    let subquery: Select<Employee> = Select::new(reports_table)
        .select_only()
        .column(report_id)
        .filter(report_manager_id.eq_col(employee_id()));

    let query: Select<Employee> = Select::new(employees_table()).where_exists(subquery);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT employees.* FROM employees WHERE EXISTS (SELECT caf\u{e9}.id FROM employees caf\u{e9} WHERE (caf\u{e9}.manager_id = employees.id))"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn exists_subquery_does_not_treat_dollar_suffix_in_utf8_alias_as_bind_token() {
    let reports_table = employees_table().with_alias("caf\u{e9}$1");
    let report_id: Column<Employee, i64> = Column::new(reports_table, "id");
    let report_manager_id: Column<Employee, i64> = Column::new(reports_table, "manager_id");

    let subquery: Select<Employee> = Select::new(reports_table)
        .select_only()
        .column(report_id)
        .filter(report_manager_id.eq_col(employee_id()));

    let query: Select<Employee> = Select::new(employees_table()).where_exists(subquery);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT employees.* FROM employees WHERE EXISTS (SELECT caf\u{e9}$1.id FROM employees caf\u{e9}$1 WHERE (caf\u{e9}$1.manager_id = employees.id))"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn exists_subquery_does_not_treat_dollar_digits_inside_quoted_identifier_as_bind_token() {
    let weird_table = Table::new("employees").with_alias("\"$1\"");
    let weird_id: Column<Employee, i64> = Column::new(weird_table, "id");
    let weird_manager_id: Column<Employee, i64> = Column::new(weird_table, "manager_id");

    let subquery: Select<Employee> = Select::new(weird_table)
        .select_only()
        .column(weird_id)
        .filter(weird_manager_id.eq_col(employee_id()));

    let query: Select<Employee> = Select::new(employees_table()).where_exists(subquery);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT employees.* FROM employees WHERE EXISTS (SELECT \"$1\".id FROM employees \"$1\" WHERE (\"$1\".manager_id = employees.id))"
    );
    assert!(sql.binds.is_empty());
}
