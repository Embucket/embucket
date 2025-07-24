use crate::models::{QueryContext, QueryResult};
use crate::service::{CoreExecutionService, ExecutionService};
use crate::utils::Config;
use core_history::entities::worksheet::Worksheet;
use core_history::history_store::{GetQueriesParams, HistoryStore};
use core_history::store::SlateDBHistoryStore;
use core_metastore::Metastore;
use core_metastore::SlateDBMetastore;
use core_metastore::models::table::TableIdent as MetastoreTableIdent;
use core_metastore::{
    Database as MetastoreDatabase, Schema as MetastoreSchema, SchemaIdent as MetastoreSchemaIdent,
    Volume as MetastoreVolume,
};
use core_utils::Db;
use datafusion::{arrow::csv::reader::Format, assert_batches_eq};
use std::sync::Arc;

#[tokio::test]
#[allow(clippy::expect_used)]
async fn test_execute_always_returns_schema() {
    let metastore = SlateDBMetastore::new_in_memory().await;
    let history_store = Arc::new(SlateDBHistoryStore::new(Db::memory().await));
    let execution_svc = CoreExecutionService::new(
        metastore.clone(),
        history_store.clone(),
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    execution_svc
        .create_session("test_session_id".to_string())
        .await
        .expect("Failed to create session");

    let columns = execution_svc
        .query(
            "test_session_id",
            "SELECT 1 AS a, 2.0 AS b, '3' AS c WHERE False",
            QueryContext::default(),
        )
        .await
        .expect("Failed to execute query")
        .column_info();
    assert_eq!(columns.len(), 3);
    assert_eq!(columns[0].r#type, "fixed");
    assert_eq!(columns[1].r#type, "fixed");
    assert_eq!(columns[2].r#type, "text");
}

#[tokio::test]
#[allow(clippy::expect_used, clippy::too_many_lines)]
async fn test_service_upload_file() {
    let metastore = SlateDBMetastore::new_in_memory().await;
    metastore
        .create_volume(
            &"test_volume".to_string(),
            MetastoreVolume::new(
                "test_volume".to_string(),
                core_metastore::VolumeType::Memory,
            ),
        )
        .await
        .expect("Failed to create volume");
    metastore
        .create_database(
            &"embucket".to_string(),
            MetastoreDatabase {
                ident: "embucket".to_string(),
                properties: None,
                volume: "test_volume".to_string(),
            },
        )
        .await
        .expect("Failed to create database");
    let schema_ident = MetastoreSchemaIdent {
        database: "embucket".to_string(),
        schema: "public".to_string(),
    };
    metastore
        .create_schema(
            &schema_ident.clone(),
            MetastoreSchema {
                ident: schema_ident,
                properties: None,
            },
        )
        .await
        .expect("Failed to create schema");

    let file_name = "test.csv";
    let table_ident = MetastoreTableIdent {
        database: "embucket".to_string(),
        schema: "public".to_string(),
        table: "target_table".to_string(),
    };

    // Create CSV data in memory
    let csv_content = "id,name,value\n1,test1,100\n2,test2,200\n3,test3,300";
    let data = csv_content.as_bytes().to_vec();

    let history_store = Arc::new(SlateDBHistoryStore::new(Db::memory().await));
    let execution_svc = CoreExecutionService::new(
        metastore.clone(),
        history_store.clone(),
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    let session_id = "test_session_id";
    execution_svc
        .create_session(session_id.to_string())
        .await
        .expect("Failed to create session");

    let csv_format = Format::default().with_header(true);
    let rows_loaded = execution_svc
        .upload_data_to_table(
            session_id,
            &table_ident,
            data.clone().into(),
            file_name,
            csv_format.clone(),
        )
        .await
        .expect("Failed to upload file");
    assert_eq!(rows_loaded, 3);

    // Verify that the file was uploaded successfully by running select * from the table
    let query = format!("SELECT * FROM {}", table_ident.table);
    let QueryResult { records, .. } = execution_svc
        .query(session_id, &query, QueryContext::default())
        .await
        .expect("Failed to execute query");

    assert_batches_eq!(
        &[
            "+----+-------+-------+",
            "| id | name  | value |",
            "+----+-------+-------+",
            "| 1  | test1 | 100   |",
            "| 2  | test2 | 200   |",
            "| 3  | test3 | 300   |",
            "+----+-------+-------+",
        ],
        &records
    );

    let rows_loaded = execution_svc
        .upload_data_to_table(session_id, &table_ident, data.into(), file_name, csv_format)
        .await
        .expect("Failed to upload file");
    assert_eq!(rows_loaded, 3);

    // Verify that the file was uploaded successfully by running select * from the table
    let query = format!("SELECT * FROM {}", table_ident.table);
    let QueryResult { records, .. } = execution_svc
        .query(session_id, &query, QueryContext::default())
        .await
        .expect("Failed to execute query");

    assert_batches_eq!(
        &[
            "+----+-------+-------+",
            "| id | name  | value |",
            "+----+-------+-------+",
            "| 1  | test1 | 100   |",
            "| 2  | test2 | 200   |",
            "| 3  | test3 | 300   |",
            "| 1  | test1 | 100   |",
            "| 2  | test2 | 200   |",
            "| 3  | test3 | 300   |",
            "+----+-------+-------+",
        ],
        &records
    );
}

#[tokio::test]
async fn test_service_create_table_file_volume() {
    let metastore = SlateDBMetastore::new_in_memory().await;

    // Create a temporary directory for the file volume
    let temp_dir = std::env::temp_dir().join("test_file_volume");
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_path = temp_dir.to_str().expect("Failed to convert path to string");
    metastore
        .create_volume(
            &"test_volume".to_string(),
            MetastoreVolume::new(
                "test_volume".to_string(),
                core_metastore::VolumeType::File(core_metastore::FileVolume {
                    path: temp_path.to_string(),
                }),
            ),
        )
        .await
        .expect("Failed to create volume");
    metastore
        .create_database(
            &"embucket".to_string(),
            MetastoreDatabase {
                ident: "embucket".to_string(),
                properties: None,
                volume: "test_volume".to_string(),
            },
        )
        .await
        .expect("Failed to create database");
    let schema_ident = MetastoreSchemaIdent {
        database: "embucket".to_string(),
        schema: "public".to_string(),
    };
    metastore
        .create_schema(
            &schema_ident.clone(),
            MetastoreSchema {
                ident: schema_ident,
                properties: None,
            },
        )
        .await
        .expect("Failed to create schema");

    let table_ident = MetastoreTableIdent {
        database: "embucket".to_string(),
        schema: "public".to_string(),
        table: "target_table".to_string(),
    };
    let history_store = Arc::new(SlateDBHistoryStore::new(Db::memory().await));
    let execution_svc = CoreExecutionService::new(
        metastore.clone(),
        history_store.clone(),
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    let session_id = "test_session_id";
    execution_svc
        .create_session(session_id.to_string())
        .await
        .expect("Failed to create session");

    let create_table_sql = format!(
        "CREATE TABLE {table_ident} (id INT, name STRING, value FLOAT) as VALUES (1, 'test1', 100.0), (2, 'test2', 200.0), (3, 'test3', 300.0)"
    );
    let QueryResult { records, .. } = execution_svc
        .query(session_id, &create_table_sql, QueryContext::default())
        .await
        .expect("Failed to create table");

    assert_batches_eq!(
        &[
            "+-------+",
            "| count |",
            "+-------+",
            "| 3     |",
            "+-------+",
        ],
        &records
    );

    let insert_sql = format!(
        "INSERT INTO {table_ident} (id, name, value) VALUES (4, 'test4', 400.0), (5, 'test5', 500.0)"
    );
    let QueryResult { records, .. } = execution_svc
        .query(session_id, &insert_sql, QueryContext::default())
        .await
        .expect("Failed to insert data");

    assert_batches_eq!(
        &[
            "+-------+",
            "| count |",
            "+-------+",
            "| 2     |",
            "+-------+",
        ],
        &records
    );
}

#[tokio::test]
#[allow(clippy::expect_used, clippy::too_many_lines)]
async fn test_query_recording() {
    let db = Db::memory().await;
    let metastore = Arc::new(SlateDBMetastore::new(db.clone()));
    let history_store = Arc::new(SlateDBHistoryStore::new(db));
    metastore
        .create_volume(
            &"test_volume".to_string(),
            MetastoreVolume::new(
                "test_volume".to_string(),
                core_metastore::VolumeType::Memory,
            ),
        )
        .await
        .expect("Failed to create volume");

    let database_name = "embucket".to_string();

    metastore
        .create_database(
            &database_name.clone(),
            MetastoreDatabase {
                ident: "embucket".to_string(),
                properties: None,
                volume: "test_volume".to_string(),
            },
        )
        .await
        .expect("Failed to create database");

    let execution_svc = CoreExecutionService::new(
        metastore.clone(),
        history_store.clone(),
        Arc::new(Config::default()),
    )
    .await
    .expect("Failed to create execution service");

    let session_id = "test_session_id";
    execution_svc
        .create_session(session_id.to_string())
        .await
        .expect("Failed to create session");

    let schema_name = "public".to_string();

    let context = QueryContext::new(Some(database_name.clone()), Some(schema_name.clone()), None);

    //Good query
    execution_svc
        .query(
            session_id,
            format!(
                "CREATE SCHEMA {}.{}",
                database_name.clone(),
                schema_name.clone()
            )
            .as_str(),
            context.clone(),
        )
        .await
        .expect("Failed to add schema");

    assert_eq!(
        1,
        history_store
            .get_queries(GetQueriesParams::default())
            .await
            .expect("Failed to get queries")
            .len()
    );

    //Failing query
    execution_svc
        .query(
            session_id,
            format!(
                "CREATE SCHEMA {}.{}",
                database_name.clone(),
                schema_name.clone()
            )
            .as_str(),
            context.clone(),
        )
        .await
        .expect_err("Failed to not add schema");

    assert_eq!(
        2,
        history_store
            .get_queries(GetQueriesParams::default())
            .await
            .expect("Failed to get queries")
            .len()
    );

    let table_name = "test1".to_string();

    //Create table queries
    execution_svc
        .query(
            session_id,
            format!(
                "create TABLE {}.{}.{}
        external_volume = ''
	    catalog = ''
	    base_location = ''
        (
	    APP_ID TEXT,
	    PLATFORM TEXT,
	    EVENT TEXT,
        TXN_ID NUMBER(38,0),
        EVENT_TIME TEXT
	    );",
                database_name.clone(),
                schema_name.clone(),
                table_name.clone()
            )
            .as_str(),
            context.clone(),
        )
        .await
        .expect("Failed to create table");

    assert_eq!(
        3,
        history_store
            .get_queries(GetQueriesParams::default())
            .await
            .expect("Failed to get queries")
            .len()
    );

    //Insert into query
    execution_svc
        .query(
            session_id,
            format!(
                "INSERT INTO {}.{}.{} (APP_ID, PLATFORM, EVENT, TXN_ID, EVENT_TIME)
        VALUES ('12345', 'iOS', 'login', '123456', '2021-01-01T00:00:00'),
               ('67890', 'Android', 'purchase', '456789', '2021-01-01T00:02:00')",
                database_name.clone(),
                schema_name.clone(),
                table_name.clone()
            )
            .as_str(),
            context.clone(),
        )
        .await
        .expect("Failed to insert into");

    assert_eq!(
        4,
        history_store
            .get_queries(GetQueriesParams::default())
            .await
            .expect("Failed to get queries")
            .len()
    );

    //With worksheet
    let worksheet = history_store
        .add_worksheet(Worksheet::new("Testing1".to_string(), String::new()))
        .await
        .expect("Failed to add worksheet");

    assert_eq!(
        0,
        history_store
            .get_queries(GetQueriesParams::default().with_worksheet_id(worksheet.clone().id))
            .await
            .expect("Failed to get queries")
            .len()
    );

    execution_svc
        .query(
            session_id,
            format!(
                "INSERT INTO {}.{}.{} (APP_ID, PLATFORM, EVENT, TXN_ID, EVENT_TIME)
        VALUES ('1234', 'iOS', 'login', '123456', '2021-01-01T00:00:00'),
               ('6789', 'Android', 'purchase', '456789', '2021-01-01T00:02:00')",
                database_name.clone(),
                schema_name.clone(),
                table_name.clone()
            )
            .as_str(),
            QueryContext::new(
                Some(database_name.clone()),
                Some(schema_name.clone()),
                Some(worksheet.clone().id),
            ),
        )
        .await
        .expect("Failed to insert into");

    assert_eq!(
        1,
        history_store
            .get_queries(GetQueriesParams::default().with_worksheet_id(worksheet.clone().id))
            .await
            .expect("Failed to get queries")
            .len()
    );
}
