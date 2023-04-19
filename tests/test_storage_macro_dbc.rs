use common::*;
mod common;

fn _get_mysql_connection_url() -> String {
    if std::env::var("MYSQL_DATABASE_URL").is_ok() {
        std::env::var("MYSQL_DATABASE_URL").unwrap()
    } else {
        "mysql://172.24.176.1/test_db?user=nociza&password=password".to_string()
    }
}

#[tokio::test]
async fn test_storage_macro_dbc_mysql(
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;

    cl.create_entry(
        "storage_macro_test:db:url",
        _get_mysql_connection_url().as_bytes(),
    )
    .await?;

    cl.create_entry(
        "storage_macro_test:db:create_db",
        b"CREATE DATABASE IF NOT EXISTS test_db" as &[u8],
    )
    .await?;

    // Create a table and insert dummy data
    cl.create_entry(
        "storage_macro_test:db:create_table",
        b"CREATE TABLE IF NOT EXISTS users (name VARCHAR(255), email VARCHAR(255))" as &[u8],
    )
    .await?;

    cl.create_entry(
        "storage_macro_test:db:insert_data",
        b"INSERT INTO users VALUES ('Alice', 'alice<at>berkeley.edu')" as &[u8],
    )
    .await?;

    cl.create_entry(
        "storage_macro_test:db:query_users",
        b"SELECT * FROM users WHERE name = ? AND email = ?" as &[u8],
    )
    .await?;

    cl.create_entry(
        "storage_macro_test:db:cleanup",
        b"DROP TABLE IF EXISTS users" as &[u8],
    )
    .await?;

    cl.read_entry("storage_macro_test:db:$dbc:create_db")
        .await?;
    cl.read_entry("storage_macro_test:db:$dbc:create_table")
        .await?;
    cl.read_entry("storage_macro_test:db:$dbc:insert_data")
        .await?;
    let query_result = cl
        .read_entry("storage_macro_test:db:$dbc:query_users:'Alice':'alice<at>berkeley.edu'")
        .await?;
    cl.read_entry("storage_macro_test:db:$dbc:cleanup").await?;

    let stringified = String::from_utf8(query_result.clone())?;
    println!("{}", stringified);
    assert_eq!(
        stringified,
        r#"{"rows":[{"values":[{"Bytes":[65,108,105,99,101]},{"Bytes":[97,108,105,99,101,60,97,116,62,98,101,114,107,101,108,101,121,46,101,100,117]}],"columns":[{"name":"name","column_type":"VARCHAR"},{"name":"email","column_type":"VARCHAR"}]}],"affected_rows":0}"#
    );

    let deserialized: rdbc2::dbc::QueryResult = serde_json::from_slice(&query_result)?;
    assert_eq!(deserialized.rows.len(), 1);

    Ok(())
}
