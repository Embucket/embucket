use std::collections::HashMap;

use crate::config::DynamoDbConfig;
use crate::error::DynamoDbPutItemSnafu;
use crate::error::Result;
use crate::error::{DynamoDbDeleteItemSnafu, DynamoDbGetItemSnafu, Error, FailedToParseJsonSnafu};
use crate::models::SessionRecord;
use aws_sdk_dynamodb::{Client, types::AttributeValue};
use serde::de::DeserializeOwned;
use snafu::ResultExt;

const PK: &str = "PK";
const SK: &str = "SK";
const ENTITY: &str = "Entity";
const DATA: &str = "Data";

#[async_trait::async_trait]
pub trait StateStore: Send + Sync {
    async fn put_new_session(&self, session_id: &str) -> Result<()>;
    async fn put_session(&self, session: SessionRecord) -> Result<()>;
    async fn get_session(&self, session_id: &str) -> Result<SessionRecord>;
    async fn delete_session(&self, session_id: &str) -> Result<()>;
    async fn update_session(&self, session: SessionRecord) -> Result<()>;
}

/// `DynamoDB` single-table client.
#[derive(Clone, Debug)]
pub struct DynamoDbStateStore {
    client: Client,
    table_name: String,
}

impl DynamoDbStateStore {
    /// Create a DynamoDB-backed statestore using environment variables.
    ///
    /// Expected variables:
    /// - `STATESTORE_TABLE_NAME`: the `DynamoDB` table name.
    /// - `AWS_ACCESS_KEY_ID`
    /// - `AWS_SECRET_ACCESS_KEY`
    /// - `AWS_REGION`
    pub async fn new_from_env() -> Result<Self> {
        let config = DynamoDbConfig::from_env()?;
        let client = config.client().await?;
        Ok(Self {
            client,
            table_name: config.table_name,
        })
    }

    /// Create a new instance from an existing `DynamoDB` client.
    pub fn new(client: Client, table_name: impl Into<String>) -> Self {
        Self {
            client,
            table_name: table_name.into(),
        }
    }

    #[must_use]
    pub fn session_id_pk(key: &str) -> String {
        format!("SESSION#{key}")
    }
}

#[async_trait::async_trait]
impl StateStore for DynamoDbStateStore {
    async fn put_new_session(&self, session_id: &str) -> Result<()> {
        self.put_session(SessionRecord::new(session_id)).await
    }

    /// Persist a session record.
    async fn put_session(&self, session: SessionRecord) -> Result<()> {
        let mut item = HashMap::new();
        let key = Self::session_id_pk(&session.session_id.clone());
        item.insert(PK.to_string(), AttributeValue::S(key.clone()));
        item.insert(SK.to_string(), AttributeValue::S(key));
        item.insert(ENTITY.to_string(), AttributeValue::S(session.entity()));
        item.insert(
            DATA.to_string(),
            AttributeValue::S(serde_json::to_string(&session).context(FailedToParseJsonSnafu)?),
        );

        if let Some(ttl) = session.ttl_seconds {
            item.insert("ttl".to_string(), AttributeValue::N(ttl.to_string()));
        }

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .context(DynamoDbPutItemSnafu)?;

        Ok(())
    }

    /// Fetch a session by id.
    async fn get_session(&self, session_id: &str) -> Result<SessionRecord> {
        let key = Self::session_id_pk(session_id);
        let item = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key(PK, AttributeValue::S(key.clone()))
            .key(SK, AttributeValue::S(key))
            .send()
            .await
            .context(DynamoDbGetItemSnafu)?
            .item
            .ok_or(Error::NotFound)?;

        deserialize_data(item)
    }

    /// Delete a session by id.
    async fn delete_session(&self, session_id: &str) -> Result<()> {
        let key = Self::session_id_pk(session_id);
        self.client
            .delete_item()
            .table_name(&self.table_name)
            .key(PK, AttributeValue::S(key.clone()))
            .key(SK, AttributeValue::S(key))
            .send()
            .await
            .context(DynamoDbDeleteItemSnafu)?;
        Ok(())
    }

    /// Update a session by replacing its stored document.
    async fn update_session(&self, session: SessionRecord) -> Result<()> {
        self.put_session(session).await
    }
}

fn deserialize_data<T: DeserializeOwned>(mut item: HashMap<String, AttributeValue>) -> Result<T> {
    let data = item
        .remove(DATA)
        .and_then(|attr| attr.as_s().ok().map(std::string::ToString::to_string))
        .ok_or(Error::MissingData)?;

    serde_json::from_str(&data).context(FailedToParseJsonSnafu)
}
