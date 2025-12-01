use super::client::{get_query_result, login, query};
use crate::models::{JsonResponse, LoginResponse, ResponseData};
use http::header;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::task::JoinHandle;
use uuid::Uuid;

pub const USER_KEY: &str = "user";
pub const PASSWORD_KEY: &str = "password";
pub const DATABASE_QUERY_PARAM_KEY: &str = "database";
pub const SCHEMA_QUERY_PARAM_KEY: &str = "schema";
pub const REQUEST_ID_KEY: &str = "request_id";
pub const SESSION_ID_KEY: &str = "session_id";
pub const ACCESS_TOKEN_KEY: &str = "access_token";

#[allow(clippy::implicit_hasher)] // disabling false positive clippy warning
pub async fn snow_sql(
    server_addr: &SocketAddr,
    sql: &str,
    params: &mut HashMap<&str, String>,
) -> (JsonResponse, Option<tokio::task::JoinHandle<()>>) {
    let client = reqwest::Client::new();
    if params.get(ACCESS_TOKEN_KEY).is_none() {
        let (headers, login_res) = login::<LoginResponse>(&client, server_addr, params.clone())
            .await
            .expect("Failed to login");
        assert_eq!(headers.get(header::WWW_AUTHENTICATE), None);

        let access_token = login_res.data.map_or_else(String::new, |data| data.token);
        params.insert(ACCESS_TOKEN_KEY, access_token);
    }
    let access_token = params
        .get(ACCESS_TOKEN_KEY)
        .expect("Access token not found");
    let request_id = Uuid::parse_str(params.get(REQUEST_ID_KEY).expect("Request ID not found"))
        .expect("Invalid request ID");

    if sql.starts_with("!result") {
        let query_id = sql.trim_start_matches("!result ");

        let (_headers, history_res) =
            get_query_result::<JsonResponse>(&client, server_addr, access_token, query_id)
                .await
                .expect("Failed to get query result");
        (history_res, None)
    } else {
        // if sql ends with ;> it is async query
        let (sql, async_exec) = if sql.ends_with(";>") {
            (sql.trim_end_matches(";>"), true)
        } else {
            (sql, false)
        };

        let sql = if sql.starts_with("!abort") {
            let query_id = sql.trim_start_matches("!abort ");
            &format!("SELECT SYSTEM$CANCEL_QUERY('{query_id}');")
        } else {
            sql
        };

        let (_headers, res) = query::<JsonResponse>(
            &client,
            server_addr,
            access_token,
            request_id,
            0,
            sql,
            async_exec,
        )
        .await
        .expect("Failed to run query");

        if async_exec {
            // spawn task to fetch results
            if let Some(ResponseData {
                query_id: Some(query_id),
                ..
            }) = res.data.as_ref()
            {
                let async_res = spawn_task_get_query_result(
                    *server_addr,
                    access_token.to_string(),
                    query_id.clone(),
                );
                return (res, Some(async_res));
            }
        }
        (res, None)
    }
}

fn spawn_task_get_query_result(
    server_addr: SocketAddr,
    access_token: String,
    query_id: String,
) -> JoinHandle<()> {
    let client = reqwest::Client::new();
    tokio::task::spawn(async move {
        // ignore result
        let _ =
            get_query_result::<JsonResponse>(&client, &server_addr, &access_token, &query_id).await;
    })
}
