use serde::{Deserialize, Serialize};
use serde_json::de;
use serde_json::ser;
use slatedb::db::Db as SlateDb;
use slatedb::db_iter::DbIterator;
use slatedb::error::SlateDBError;
use snafu::prelude::*;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bytes::Bytes;
use std::i64;
use std::ops::RangeBounds;

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("SlateDB error: {source}"))]
    Database { source: SlateDBError },

    #[snafu(display("SlateDB error: {source} while fetching key {key}"))]
    KeyGet { key: String, source: SlateDBError },

    #[snafu(display("SlateDB error: {source} while scanning range {start_key}..{end_key}"))]
    Scan { start_key: String, end_key: String, source: SlateDBError },

    #[snafu(display("Error serializing value: {source}"))]
    SerializeValue { source: serde_json::Error },

    #[snafu(display("Error deserializing value: {source}"))]
    DeserializeValue { source: serde_json::Error },

    #[snafu(display("Key Not found"))]
    KeyNotFound,
}

type Result<T> = std::result::Result<T, Error>;

pub type QueryHistoryResult<T> = Result<T>;

pub trait QueryHistoryEntity {
    fn id(&self) -> Uuid;
    fn key_from_time(time: DateTime<Utc>) -> String;
    fn key(&self) -> String;
    fn prefix() -> &'static str;

    fn min_key() -> String {
        format!("{}{}", Self::prefix(), 0)
    }
    fn max_key() -> String {
        format!("{}{}", Self::prefix(), i64::MAX)
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SomeItem {
    pub id: Uuid,
    pub query: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status_code: u16,
}



impl QueryHistoryEntity for SomeItem {
    fn id(&self) -> Uuid {
        self.id
    }

    fn prefix() -> &'static str {
        "si."
    }

    fn key_from_time(time: DateTime<Utc>) -> String {
        format!("{}{}", Self::prefix(), time.to_string())
    }

    fn key(&self) -> String {
        Self::key_from_time(self.start_time)
    }
}



#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: Uuid,
    pub query: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status_code: u16,
}

impl QueryHistoryEntity for HistoryItem {
    fn id(&self) -> Uuid {
        self.id
    }

    fn prefix() -> &'static str {
        "hi."
    }

    fn key_from_time(time: DateTime<Utc>) -> String {
        format!("{}{}", Self::prefix(), time.to_string())
    }

    fn key(&self) -> String {
        Self::key_from_time(self.start_time)
    }
}

// Kind of cast for range, for cases when for range 
// trait `RangeBounds<bytes::Bytes>` is not implemented.
macro_rules! RangeAsRef {
    { $range: ident } => {
        (
            $range
            .start_bound()
            .map(|b| Bytes::copy_from_slice(b.as_ref())),
            $range
            .end_bound()
            .map(|b| Bytes::copy_from_slice(b.as_ref()))
        )
    }
}

// To be used with the RangeFull
macro_rules! RangeFull {
    { $range: ident } => {
        (
            $range
            .start_bound()
            .map(|b| Bytes::copy_from_slice(b)),
            $range
            .end_bound()
            .map(|b| Bytes::copy_from_slice(b))
        )
    }
}

pub struct DbWrapper(SlateDb);

impl DbWrapper {
    pub const fn new(db: SlateDb) -> Self {
        Self(db)
    }

    /// Closes the database connection.
    ///
    /// # Errors
    ///
    /// Returns a `DbError` if the underlying database operation fails.
    pub async fn close(&self) -> Result<()> {
        return self.0.close().await.context(DatabaseSnafu)
    }

    /// Stores template object in the database.
    ///
    /// # Errors
    ///
    /// Returns a `SerializeError` if the value cannot be serialized to JSON.
    /// Returns a `DbError` if the underlying database operation fails.
    pub async fn put<T: serde::Serialize + Sync + QueryHistoryEntity>(&self, entity: &T) -> Result<()>
    {
        let serialized = ser::to_vec(entity).context(SerializeValueSnafu)?;
        self.0.put(entity.key().as_bytes(), serialized.as_ref()).await.context(DatabaseSnafu)
    }

    /// Iterator For iterating on range
    ///
    /// # Errors
    ///
    /// Returns a `DbError` if the underlying database operation fails.
    pub async fn range_iterator<K, T>(&self, range: T) -> Result<DbIterator<'_>>
    where
        K: AsRef<[u8]>,
        T: RangeBounds<K>,
    {
        self.0.scan(RangeAsRef!(range)).await.context(DatabaseSnafu)
    }

    /// Fetch items from database
    ///
    /// # Errors
    ///
    /// Returns a `DeserializeError` if the value cannot be serialized to JSON.
    /// Returns a `DbError` if the underlying database operation fails.    
    pub async fn items_from_range<K, R, T: for<'de> serde::de::Deserialize<'de> + Sync + QueryHistoryEntity>(&self, range: R, limit: Option<u16>) -> Result<Vec<T>> 
    where
        K: AsRef<[u8]>,
        R: RangeBounds<K>,
    {
        let mut iter = self.range_iterator(range).await?;
        let mut items: Vec<T> = vec![];
        while let Ok(Some(item)) = iter.next().await {
            let item = de::from_slice(&item.value).context(DeserializeValueSnafu)?;
            items.push(item);
            if items.len() >= limit.unwrap_or(u16::MAX).into() {
                break;
            }
        }
        Ok(items)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb::config::DbOptions;
    use slatedb::db::Db as SlateDb;
    use std::sync::Arc;
    use chrono::{DateTime, TimeZone, Utc};
    use tokio;
    use bytes::Bytes;
    use std::time::SystemTime;
    use futures::future::join_all;


    async fn create_slate_db() -> DbWrapper {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let options = DbOptions::default();
        DbWrapper::new(
            SlateDb::open_with_opts(Path::from("/tmp/test_kv_store"), options, object_store)
                .await
                .unwrap(),
        )
    }

    fn new_history_item(prev: Option<HistoryItem>) -> HistoryItem {
        let ts = match prev {
            Some(item) => item.end_time.timestamp(),
            _ => {
                Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap().timestamp()
            }
        };
        let start_time = DateTime::from_timestamp(ts, 0).unwrap();
        let end_time = DateTime::from_timestamp(ts+60*60*24, 0).unwrap();
        HistoryItem {
            id: Uuid::new_v4(),
            query: String::from(format!("SELECT {start_time}")),
            start_time: start_time,
            end_time: end_time,
            status_code: 200,
        }
    }

    async fn create_populate_new_db() -> (Arc<DbWrapper>, Vec<HistoryItem>) {
        let db = Arc::new(create_slate_db().await);
        let mut item: Option<HistoryItem> = None;
        
        let started = SystemTime::now();
        println!("Create QueryHistory items {:?}", SystemTime::now().duration_since(started));

        const COUNT: usize = 100;
        let mut items: Vec<HistoryItem> = vec![];
        for _ in 0..COUNT {
            item = Some(new_history_item(item));
            items.push(item.clone().unwrap());
        }
        println!("{} QueryHistory items created {:?}", COUNT, SystemTime::now().duration_since(started));

        let mut fut = Vec::new();
        for item in items.iter() {
            fut.push(db.put(item))
        }
        join_all(fut).await;
        println!(
            "Added QueryHistory items count={} in {:?}", 
            COUNT, SystemTime::now().duration_since(started)
        );

        let mut iter =  db.0.scan(..).await.unwrap();
        let mut i = 0;
        while let Ok(Some(item)) = iter.next().await {
            assert_eq!(
                item.key,
                Bytes::from(items[i].key())
            );
            assert_eq!(
                item.value,
                Bytes::from(ser::to_string(&items[i]).context(SerializeValueSnafu).unwrap())
            );
            i += 1;
        }
        assert_eq!(i, items.len());

        (db, items)
    }

    async fn populate_some_items(db: &Arc<DbWrapper>) -> Vec<SomeItem>{
        let items = vec![
            SomeItem{
                id: Uuid::new_v4(),
                query: "SELECT 1".to_string(),
                start_time: DateTime::from_timestamp(Utc::now().timestamp(), 0).unwrap(),
                end_time: DateTime::from_timestamp(Utc::now().timestamp(), 0).unwrap(),
                status_code: 0,
            },
            SomeItem{
                id: Uuid::new_v4(),
                query: "SELECT 2".to_string(),
                start_time: DateTime::from_timestamp(Utc::now().timestamp(), 1).unwrap(),
                end_time: DateTime::from_timestamp(Utc::now().timestamp(), 1).unwrap(),
                status_code: 0,
            }
        ];
        for item in items.iter() {
            let _res = db.put(item).await;
        }
        
        items
    }

    fn assert_check_items<T: serde::Serialize + Sync + QueryHistoryEntity>(created_items: Vec<&T>, retrieved_items: Vec<&T>) {
        assert_eq!(created_items.len(), retrieved_items.len());
        assert_eq!(
            created_items.last().unwrap().key(),
            retrieved_items.last().unwrap().key(),
        );
        for (i, item) in created_items.iter().enumerate() {
            assert_eq!(
                Bytes::from(ser::to_string(&item).context(SerializeValueSnafu).unwrap()),
                Bytes::from(ser::to_string(&retrieved_items[i]).context(SerializeValueSnafu).unwrap()),
            );    
        }
    }

    #[tokio::test]
    // test keys groups having different prefixes for separate ranges
    async fn test_slatedb_separate_keys_groups() {
        let (db, created_history_items) = create_populate_new_db().await;
        let created_some_items = populate_some_items(&db).await;

        let created = created_history_items;
        let range = created.first().unwrap().key()..=created.last().unwrap().key();
        println!("HistoryItem range {range:?}");
        let retrieved: Vec<HistoryItem> = db.items_from_range(range, None).await.unwrap();
        assert_check_items(created.iter().collect(), retrieved.iter().collect());

        let created = created_some_items;
        let range = created.first().unwrap().key()..=created.last().unwrap().key();
        println!("SomeItem range {range:?}");
        let retrieved: Vec<SomeItem> = db.items_from_range(range, None).await.unwrap();
        assert_check_items(created.iter().collect(), retrieved.iter().collect());
    }

    #[tokio::test]
    // test key groups having different prefixes
    async fn test_slatedb_separate_key_groups_within_min_max_range() {
        let (db, created_history_items) = create_populate_new_db().await;
        let created_some_items = populate_some_items(&db).await;

        let range = HistoryItem::min_key()..HistoryItem::max_key();
        println!("HistoryItem range {range:?}");
        let retrieved: Vec<HistoryItem> = db.items_from_range(range, None).await.unwrap();
        assert_check_items(created_history_items.iter().collect(), retrieved.iter().collect());
        
        let range = SomeItem::min_key()..SomeItem::max_key();
        println!("SomeItem range {range:?}");
        let retrieved: Vec<SomeItem> = db.items_from_range(range, None).await.unwrap();
        assert_check_items(created_some_items.iter().collect(), retrieved.iter().collect());
    }

    #[tokio::test]
    // test keys groups having different prefixes for separate ranges
    async fn test_slatedb_limit() {
        let (db, created_history_items) = create_populate_new_db().await;
        let created = created_history_items;
        let range = created.first().unwrap().key()..=created.last().unwrap().key();
        let limit: usize = 10;
        println!("HistoryItem range {range:?}, limit {limit}");
        let retrieved: Vec<HistoryItem> = db.items_from_range(range, Some(limit as u16)).await.unwrap();
        assert_check_items(created[0..limit].iter().collect(), retrieved.iter().collect());
    }

    #[tokio::test]
    async fn test_slatedb_start_with_existing_key_end_with_max_key_range() {
        let (db, created_items) = create_populate_new_db().await;
        let items: Vec<&HistoryItem> = created_items[5..].into_iter().collect();
        let range = items.first().unwrap().key()..HistoryItem::max_key();
        let retrieved: Vec<HistoryItem> = db.items_from_range(range, None).await.unwrap();
        assert_check_items(items, retrieved.iter().collect());
    }

    #[tokio::test]
    // test full range .. and how all the items retrieved
    async fn test_slatedb_dont_distinguish_key_groups_within_full_range() {
        let (db, created_history_items) = create_populate_new_db().await;
        let created_some_items = populate_some_items(&db).await;

        let range = ..;
        let retrieved: Vec<HistoryItem> = db.items_from_range(RangeFull!(range), None).await.unwrap();
        assert_eq!(created_history_items.len() + created_some_items.len(), retrieved.len());
        assert_ne!(retrieved.first().unwrap().key(), retrieved.last().unwrap().key());
    }
}
