use crate::tests::sql_test_macro::{SqlTest, sql_test_wrapper};
use crate::server::core_state::MetastoreConfig;

mod stress {
    use super::*;
    
    #[tokio::test(flavor = "multi_thread")]
    async fn concurrency_test_memory_database() {
        let handles = (0..50).map(|idx| {
            tokio::spawn(async move {
                sql_test_wrapper(
                    SqlTest::new(&[
                        "create table if not exists embucket.public.test_table (id int)",
                        "drop table if exists embucket.public.test_table",
                    ])
                //.with_metastore_config(MetastoreConfig::DefaultConfig)
                .with_metastore_config(MetastoreConfig::ConfigPath("/home/yaroslav/git/embucket/config/metastore.yaml".into()))
                .with_skip_login(),
                move |sql_info, response| {
                    let sql = sql_info.0;
                    let err_msg = response.message.clone().unwrap_or_default();
                    let err_code = response.code.clone().unwrap_or_default();
                    println!("{idx}: {sql} = {err_msg} {err_code}");
                    response.code.is_none()
                }).await;
            })
        }).collect::<Vec<_>>();
        futures::future::join_all(handles).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrency_test_s3tables_database() {
        let handles = (0..50).map(|idx| {
            tokio::spawn(async move {
                sql_test_wrapper(
                    SqlTest::new(&[
                        "create table if not exists my_s3_table_bucket.schema1.test_table (id int)",
                        // "drop table if exists my_s3_table_bucket.schema1.test_table",
                    ])
                //.with_metastore_config(MetastoreConfig::DefaultConfig)
                .with_metastore_config(MetastoreConfig::ConfigPath("/home/yaroslav/git/embucket/config/metastore.yaml".into())),
                move |sql_info, response| {
                    let sql = sql_info.0;
                    let err_msg = response.message.clone().unwrap_or_default();
                    let err_code = response.code.clone().unwrap_or_default();
                    println!("{idx}: {sql} = {err_msg} {err_code}");
                    true
                }).await;
            })
        }).collect::<Vec<_>>();
        futures::future::join_all(handles).await;

        assert!(false);
    }    
}