use crate::StateStore;
use crate::config::DynamoDbConfig;
use crate::error::Result;
use crate::error::{
    DynamoDbDeleteItemSnafu, DynamoDbGetItemSnafu, DynamoDbPutItemSnafu, DynamoDbQueryOutputSnafu,
    Error, FailedToDeserializeDynamoSnafu, FailedToSerializeDynamoSnafu,
};
use crate::models::{Query, SessionRecord};
use aws_sdk_dynamodb::{Client, types::AttributeValue};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_dynamo::{from_item, to_item};
use snafu::ResultExt;
use std::collections::HashMap;

const PK: &str = "PK";
const SK: &str = "SK";
const ENTITY: &str = "Entity";
const QUERY_ID: &str = "query_id";
const REQUEST_ID: &str = "request_id";
const SESSION_ID: &str = "session_id";
const QUERY_ID_INDEX: &str = "GSI_QUERY_ID_INDEX";
const REQUEST_ID_INDEX: &str = "GSI_REQUEST_ID_INDEX";
const SESSION_ID_INDEX: &str = "GSI_SESSION_ID_INDEX";

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

    fn query_pk(start_time: &DateTime<Utc>) -> String {
        format!("QUERY#{}", start_time.format("%Y-%m-%d"))
    }

    fn query_sk(start_time: &DateTime<Utc>) -> String {
        start_time.timestamp_millis().to_string()
    }

    async fn query_item_by_query_id(
        &self,
        query_id: &str,
    ) -> Result<HashMap<String, AttributeValue>> {
        let items = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name(QUERY_ID_INDEX)
            .key_condition_expression("#query_id = :query_id")
            .expression_attribute_names("#query_id", QUERY_ID)
            .expression_attribute_values(":query_id", AttributeValue::S(query_id.to_string()))
            .send()
            .await
            .context(DynamoDbQueryOutputSnafu)?
            .items
            .unwrap_or_default();

        items.into_iter().next().ok_or(Error::NotFound)
    }

    async fn query_item_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<HashMap<String, AttributeValue>> {
        let items = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name(REQUEST_ID_INDEX)
            .key_condition_expression("#request_id = :request_id")
            .expression_attribute_names("#request_id", REQUEST_ID)
            .expression_attribute_values(":request_id", AttributeValue::S(request_id.to_string()))
            .send()
            .await
            .context(DynamoDbQueryOutputSnafu)?
            .items
            .unwrap_or_default();

        items.into_iter().next().ok_or(Error::NotFound)
    }

    async fn query_items_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<Vec<HashMap<String, AttributeValue>>> {
        let items = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name(SESSION_ID_INDEX)
            .key_condition_expression("#session_id = :session_id")
            .expression_attribute_names("#session_id", SESSION_ID)
            .expression_attribute_values(":session_id", AttributeValue::S(session_id.to_string()))
            .send()
            .await
            .context(DynamoDbQueryOutputSnafu)?
            .items
            .unwrap_or_default();

        Ok(items)
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
        item.extend(model_attributes(&session)?);

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

        deserialize_item(item)
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

    async fn put_query(&self, query: &Query) -> Result<()> {
        let mut item = HashMap::new();
        let pk = Self::query_pk(&query.start_time);
        let sk = Self::query_sk(&query.start_time);
        item.insert(PK.to_string(), AttributeValue::S(pk));
        item.insert(SK.to_string(), AttributeValue::S(sk));
        item.insert(ENTITY.to_string(), AttributeValue::S(query.entity()));
        item.extend(model_attributes(query)?);
        item.insert(
            QUERY_ID.to_string(),
            AttributeValue::S(query.query_id.to_string()),
        );
        item.insert(
            SESSION_ID.to_string(),
            AttributeValue::S(query.session_id.clone()),
        );
        if let Some(request_id) = &query.request_id {
            item.insert(
                REQUEST_ID.to_string(),
                AttributeValue::S(request_id.to_string()),
            );
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

    async fn get_query(&self, query_id: &str) -> Result<Query> {
        let item = self.query_item_by_query_id(query_id).await?;
        deserialize_item(item)
    }

    async fn get_query_by_request_id(&self, request_id: &str) -> Result<Query> {
        let item = self.query_item_by_request_id(request_id).await?;
        deserialize_item(item)
    }

    async fn get_queries_by_session_id(&self, session_id: &str) -> Result<Vec<Query>> {
        let items = self.query_items_by_session_id(session_id).await?;
        deserialize_items(items)
    }

    async fn delete_query(&self, query_id: &str) -> Result<()> {
        let item = self.query_item_by_query_id(query_id).await?;
        let pk = required_string_attr(&item, PK)?;
        let sk = required_string_attr(&item, SK)?;
        self.client
            .delete_item()
            .table_name(&self.table_name)
            .key(PK, AttributeValue::S(pk))
            .key(SK, AttributeValue::S(sk))
            .send()
            .await
            .context(DynamoDbDeleteItemSnafu)?;
        Ok(())
    }

    async fn update_query(&self, query: &Query) -> Result<()> {
        self.put_query(query).await
    }
}

fn model_attributes<T: Serialize>(value: &T) -> Result<HashMap<String, AttributeValue>> {
    to_item(value).context(FailedToSerializeDynamoSnafu)
}

fn deserialize_item<T: DeserializeOwned>(mut item: HashMap<String, AttributeValue>) -> Result<T> {
    item.remove(PK);
    item.remove(SK);
    item.remove(ENTITY);
    item.remove("ttl");
    from_item(item).context(FailedToDeserializeDynamoSnafu)
}

fn deserialize_items<T: DeserializeOwned>(
    items: Vec<HashMap<String, AttributeValue>>,
) -> Result<Vec<T>> {
    items
        .into_iter()
        .map(deserialize_item)
        .collect::<Result<Vec<_>>>()
}

fn required_string_attr(item: &HashMap<String, AttributeValue>, key: &str) -> Result<String> {
    item.get(key)
        .and_then(|attr| attr.as_s().ok())
        .map(std::string::ToString::to_string)
        .ok_or(Error::MissingData)
}
