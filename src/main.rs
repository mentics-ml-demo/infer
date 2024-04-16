
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let uri = std::env::var("SCYLLA_URI").unwrap_or_else(|_| "127.0.0.1:9042".to_string());
    println!("Connecting to ScyllaDB at {uri}");
    let session = create_session(&uri).await?;

    println!("Adding key-values");
    let kv1 = KeyValue { id: 1, time: 100, datum1: 11.01, datum2: 12.0 };
    let kv2 = KeyValue { id: 2, time: 103, datum1: 12.20, datum2: 4.2323 };
    let kv3 = KeyValue { id: 3, time: 107, datum1: 9.50, datum2: 3.000001111 };
    add_key_value(&session, kv1).await?;
    add_key_value(&session, kv2).await?;
    add_key_value(&session, kv3).await?;

    println!("Selecting key-values");
    let kvs = select_key_value(&session, 2).await?;
    println!("Found {:?}", kvs);
    Ok(())
}


use scylla::{IntoTypedRows, SerializeRow, FromRow, ValueList, Session, SessionBuilder};

pub async fn create_session(uri: &str) -> Result<Session> {
    SessionBuilder::new()
        .known_node(uri)
        .build()
        .await
        .map_err(From::from)
}


#[derive(Debug, FromRow, ValueList, SerializeRow)]
pub struct KeyValue {
    pub id: i64,
    pub time: i64,
    pub datum1: f32,
    pub datum2: f32
}


static ADD_KEY_VALUE: &str = r#"
    INSERT INTO test_kv.kv (id, time, datum1, datum2) VALUES (?, ?, ?, ?);
"#;

pub async fn add_key_value(session: &Session, kv: KeyValue) -> Result<()> {
    session
        .query(ADD_KEY_VALUE, kv)
        .await
        .map(|_| ())
        .map_err(From::from)
}



static SELECT_KEY_VALUE: &str = r#"
    SELECT id, time, datum1, datum2 FROM test_kv.kv WHERE id = ?;
"#;

pub async fn select_key_value(
    session: &Session,
    id: i64
) -> Result<Vec<KeyValue>> {
    session
        .query(SELECT_KEY_VALUE, (id,))
        .await?
        .rows
        .unwrap_or_default()
        .into_typed::<KeyValue>()
        .map(|v| v.map_err(From::from))
        .collect()
}
