#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::http::error::ErrorResponse;
use crate::http::ui::tests::common::http_req;
use crate::http::ui::worksheets::{
    SortBy, SortOrder, Worksheet, WorksheetCreatePayload, WorksheetUpdatePayload,
    WorksheetsResponse,
};
use crate::tests::run_test_server;
use http::Method;
use reqwest;
use serde_json::json;
use std::net::SocketAddr;

async fn create_worksheets(
    client: &reqwest::Client,
    addr: &SocketAddr,
    templates: Vec<(&str, &str)>,
) -> Vec<Worksheet> {
    let mut created: Vec<Worksheet> = vec![];
    for (name, query) in templates {
        eprintln!("Creating worksheet: {name}, {query}");
        let worksheet = http_req::<Worksheet>(
            client,
            Method::POST,
            &format!("http://{addr}/ui/worksheets"),
            json!(WorksheetCreatePayload {
                name: String::from(name),
                content: String::from(query),
            })
            .to_string(),
        )
        .await
        .expect("Failed to create worksheet");

        assert!(worksheet.id > 0);
        created.push(worksheet);
    }
    created
}

async fn get_worksheets(
    client: &reqwest::Client,
    addr: &SocketAddr,
    sort_order: SortOrder,
    sort_by: SortBy,
) -> Vec<Worksheet> {
    http_req::<WorksheetsResponse>(
        client,
        Method::GET,
        &format!("http://{addr}/ui/worksheets?sortBy={sort_by}&sortOrder={sort_order}"),
        String::new(),
    )
    .await
    .expect("Failed to get worksheets")
    .items
}

async fn update_worksheet(
    client: &reqwest::Client,
    addr: &SocketAddr,
    id: i64,
    payload: WorksheetUpdatePayload,
) {
    http_req::<()>(
        client,
        Method::PATCH,
        &format!("http://{addr}/ui/worksheets/{id}"),
        json!(payload).to_string(),
    )
    .await
    .expect("Failed to update worksheet");
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_ui_worksheets_sort() {
    let addr = run_test_server().await;
    let client = reqwest::Client::new();

    let templates = vec![
        ("name1", ""),
        ("name2", "select 2"),
        ("name3", ""),
        ("name4", "select 4"),
    ];
    let created = create_worksheets(&client, &addr, templates).await;

    // check sort by name
    let sort_by_name_asc = get_worksheets(&client, &addr, SortOrder::Ascending, SortBy::Name).await;
    assert_eq!(
        vec!["name1", "name2", "name3", "name4"],
        sort_by_name_asc
            .into_iter()
            .map(|w| w.name)
            .collect::<Vec<String>>(),
    );

    let sort_by_name_desc =
        get_worksheets(&client, &addr, SortOrder::Descending, SortBy::Name).await;
    assert_eq!(
        vec!["name4", "name3", "name2", "name1"],
        sort_by_name_desc
            .into_iter()
            .map(|w| w.name)
            .collect::<Vec<String>>(),
    );

    // check sort by created_at
    let sort_by_created_at_asc =
        get_worksheets(&client, &addr, SortOrder::Ascending, SortBy::CreatedAt).await;
    assert_eq!(
        vec!["name1", "name2", "name3", "name4"],
        sort_by_created_at_asc
            .into_iter()
            .map(|w| w.name)
            .collect::<Vec<String>>(),
    );
    let sort_by_created_at_desc =
        get_worksheets(&client, &addr, SortOrder::Descending, SortBy::CreatedAt).await;
    assert_eq!(
        vec!["name4", "name3", "name2", "name1"],
        sort_by_created_at_desc
            .into_iter()
            .map(|w| w.name)
            .collect::<Vec<String>>(),
    );

    // check sort by updated_at
    let sort_by_updated_at_asc =
        get_worksheets(&client, &addr, SortOrder::Ascending, SortBy::UpdatedAt).await;
    assert_eq!(
        vec!["name1", "name2", "name3", "name4"],
        sort_by_updated_at_asc
            .into_iter()
            .map(|w| w.name)
            .collect::<Vec<String>>(),
    );
    let sort_by_updated_at_desc =
        get_worksheets(&client, &addr, SortOrder::Descending, SortBy::UpdatedAt).await;
    assert_eq!(
        vec!["name4", "name3", "name2", "name1"],
        sort_by_updated_at_desc
            .into_iter()
            .map(|w| w.name)
            .collect::<Vec<String>>(),
    );

    // update worksheets
    update_worksheet(
        &client,
        &addr,
        created[0].id,
        WorksheetUpdatePayload {
            name: Some("name91-updated".to_string()),
            content: None,
        },
    )
    .await;
    update_worksheet(
        &client,
        &addr,
        created[2].id,
        WorksheetUpdatePayload {
            name: Some("name31-updated".to_string()),
            content: None,
        },
    )
    .await;

    // check sort by created_at after update (nothing changed)
    let upd_sort_by_created_at_asc =
        get_worksheets(&client, &addr, SortOrder::Ascending, SortBy::CreatedAt).await;
    assert_eq!(
        vec!["name91-updated", "name2", "name31-updated", "name4"],
        upd_sort_by_created_at_asc
            .into_iter()
            .map(|w| w.name)
            .collect::<Vec<String>>(),
    );
    let upd_sort_by_created_at_desc =
        get_worksheets(&client, &addr, SortOrder::Descending, SortBy::CreatedAt).await;
    assert_eq!(
        vec!["name4", "name31-updated", "name2", "name91-updated"],
        upd_sort_by_created_at_desc
            .into_iter()
            .map(|w| w.name)
            .collect::<Vec<String>>(),
    );

    // check sort by name after update
    let upd_sort_by_name_asc =
        get_worksheets(&client, &addr, SortOrder::Ascending, SortBy::Name).await;
    assert_eq!(
        vec!["name2", "name31-updated", "name4", "name91-updated"],
        upd_sort_by_name_asc
            .into_iter()
            .map(|w| w.name)
            .collect::<Vec<String>>(),
    );
    let upd_sort_by_name_desc =
        get_worksheets(&client, &addr, SortOrder::Descending, SortBy::Name).await;
    assert_eq!(
        vec!["name91-updated", "name4", "name31-updated", "name2"],
        upd_sort_by_name_desc
            .into_iter()
            .map(|w| w.name)
            .collect::<Vec<String>>(),
    );

    // check sort by updated_at after update
    let upd_sort_by_updated_at_asc =
        get_worksheets(&client, &addr, SortOrder::Ascending, SortBy::UpdatedAt).await;
    assert_eq!(
        vec!["name2", "name4", "name91-updated", "name31-updated"],
        upd_sort_by_updated_at_asc
            .into_iter()
            .map(|w| w.name)
            .collect::<Vec<String>>(),
    );
    let upd_sort_by_updated_at_desc =
        get_worksheets(&client, &addr, SortOrder::Descending, SortBy::UpdatedAt).await;
    assert_eq!(
        vec!["name31-updated", "name91-updated", "name4", "name2"],
        upd_sort_by_updated_at_desc
            .into_iter()
            .map(|w| w.name)
            .collect::<Vec<String>>(),
    );
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_ui_worksheets() {
    let addr = run_test_server().await;
    let client = reqwest::Client::new();

    let resp = http_req::<()>(
        &client,
        Method::DELETE,
        &format!("http://{addr}/ui/worksheets"),
        String::new(),
    )
    .await
    .expect_err("Should fail with METHOD_NOT_ALLOWED");
    assert_eq!(http::StatusCode::METHOD_NOT_ALLOWED, resp.status);

    let worksheet1 = http_req::<Worksheet>(
        &client,
        Method::POST,
        &format!("http://{addr}/ui/worksheets"),
        json!(WorksheetCreatePayload {
            name: String::new(),
            content: String::new(),
        })
        .to_string(),
    )
    .await
    .expect("Failed to create worksheet");

    assert!(worksheet1.id > 0);
    assert!(!worksheet1.name.is_empty()); // test behavior: name based on time

    let create_payload = WorksheetCreatePayload {
        name: "test".to_string(),
        content: "select 1;".to_string(),
    };

    let worksheet2 = http_req::<Worksheet>(
        &client,
        Method::POST,
        &format!("http://{addr}/ui/worksheets"),
        json!(create_payload).to_string(),
    )
    .await
    .expect("Failed to create worksheet");

    assert!(worksheet2.id > 0);
    assert_eq!(worksheet2.name, create_payload.name);
    assert_eq!(worksheet2.content, create_payload.content);

    let worksheets = http_req::<WorksheetsResponse>(
        &client,
        Method::GET,
        &format!("http://{addr}/ui/worksheets"),
        String::new(),
    )
    .await
    .expect("Failed to get worksheets")
    .items;
    assert_eq!(worksheets.len(), 2);
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_ui_worksheets_ops() {
    let addr = run_test_server().await;
    let client = reqwest::Client::new();

    // bad payload, None instead of string
    let res = http_req::<()>(
        &client,
        Method::POST,
        &format!("http://{addr}/ui/worksheets"),
        json!(WorksheetUpdatePayload {
            name: None,
            content: None,
        })
        .to_string(),
    )
    .await
    .expect_err("Should fail with UNPROCESSABLE_ENTITY");
    assert_eq!(http::StatusCode::UNPROCESSABLE_ENTITY, res.status);

    let worksheet = http_req::<Worksheet>(
        &client,
        Method::POST,
        &format!("http://{addr}/ui/worksheets"),
        json!(WorksheetCreatePayload {
            name: String::new(),
            content: String::new(),
        })
        .to_string(),
    )
    .await
    .expect("Failed to create worksheet");

    assert!(!worksheet.name.is_empty());

    let res = http_req::<()>(
        &client,
        Method::GET,
        &format!("http://{addr}/ui/worksheets/{}", 0),
        String::new(),
    )
    .await
    .expect_err("Should fail with NOT_FOUND");
    assert_eq!(http::StatusCode::NOT_FOUND, res.status);
    let error_response = serde_json::from_str::<ErrorResponse>(&res.body)
        .expect("Failed to serialize error response");
    assert_eq!(http::StatusCode::NOT_FOUND, error_response.status_code);

    let _ = http_req::<Worksheet>(
        &client,
        Method::GET,
        &format!("http://{addr}/ui/worksheets/{}", worksheet.id),
        String::new(),
    )
    .await
    .expect("Failed to get worksheet");

    let patch_payload = WorksheetUpdatePayload {
        name: Some("test".to_string()),
        content: Some("select 1".to_string()),
    };
    let resp = http_req::<()>(
        &client,
        Method::PATCH,
        &format!("http://{addr}/ui/worksheets/{}", 0),
        json!(patch_payload).to_string(),
    )
    .await
    .expect_err("Should fail with NOT_FOUND");
    assert_eq!(http::StatusCode::NOT_FOUND, resp.status);

    http_req::<()>(
        &client,
        Method::PATCH,
        &format!("http://{addr}/ui/worksheets/{}", worksheet.id),
        json!(patch_payload).to_string(),
    )
    .await
    .expect("Failed to update worksheet");

    let worksheet_2 = http_req::<Worksheet>(
        &client,
        Method::GET,
        &format!("http://{addr}/ui/worksheets/{}", worksheet.id),
        json!(patch_payload).to_string(),
    )
    .await
    .expect("Failed to get worksheet");
    assert_eq!(Some(worksheet_2.name), patch_payload.name);
    assert_eq!(Some(worksheet_2.content), patch_payload.content);

    http_req::<()>(
        &client,
        Method::DELETE,
        &format!("http://{addr}/ui/worksheets/{}", worksheet.id),
        String::new(),
    )
    .await
    .expect("Failed to delete worksheet");

    // Shouldn't exist
    let resp = http_req::<()>(
        &client,
        Method::DELETE,
        &format!("http://{addr}/ui/worksheets/{}", worksheet.id),
        String::new(),
    )
    .await
    .expect_err("Should fail with NOT_FOUND");
    assert_eq!(http::StatusCode::NOT_FOUND, resp.status);
}
