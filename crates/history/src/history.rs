use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::de;
use serde_json::ser;
use slatedb::db::Db as SlateDb;
use slatedb::db_iter::DbIterator;
use slatedb::error::SlateDBError;
use snafu::prelude::*;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bytes::Bytes;
use std::ops::RangeBounds;

#[derive(Snafu, Debug)]
//#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("SlateDB error: {source}"))]
    Database { source: SlateDBError },

    #[snafu(display("SlateDB error: {source} while fetching key {key}"))]
    KeyGet { key: String, source: SlateDBError },

    #[snafu(display("SlateDB error: {source} while scanning range {start_key}..{end_key}"))]
    Scan { start_key: String, end_key: String, source: SlateDBError },

    #[snafu(display("Error serializing value: {source}"))]
    SerializeValue { source: serde_json::Error },

    #[snafu(display("Deserialize error: {source}"))]
    DeserializeValue { source: serde_json::Error },

    #[snafu(display("Key Not found"))]
    KeyNotFound,
}

type Result<T> = std::result::Result<T, Error>;

pub type QueryHistoryResult<T> = Result<T>;

pub trait Entity {
    fn id(&self) -> Uuid;
    fn prefix() -> &'static str;
    fn key_from_time(time: DateTime<Utc>) -> String;
    fn key(&self) -> String;
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



impl Entity for SomeItem {
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

impl Entity for HistoryItem {
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

// Kind of cast for range, for cases if for range 
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

// Retrieve keys bounds from range, and return as Bytes array
// as trait `RangeBounds<bytes::Bytes>` is not implemented for Range
macro_rules! Range {
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

    /// Deletes a key-value pair from the database.
    ///
    /// # Errors
    ///
    /// This function will return a `DbError` if the underlying database operation fails.
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.0.delete(key.as_bytes()).await.context(DatabaseSnafu)
    }

    /// Stores a key-value pair in the database.
    ///
    /// # Errors
    ///
    /// Returns a `SerializeError` if the value cannot be serialized to JSON.
    /// Returns a `DbError` if the underlying database operation fails.
    pub async fn _put<T: serde::Serialize + Sync>(&self, key: &str, value: &T) -> Result<()> {
        let serialized = ser::to_vec(value).context(SerializeValueSnafu)?;
        self.0.put(key.as_bytes(), serialized.as_ref()).await.context(DatabaseSnafu)
    }

    /// Stores template object in the database.
    ///
    /// # Errors
    ///
    /// Returns a `SerializeError` if the value cannot be serialized to JSON.
    /// Returns a `DbError` if the underlying database operation fails.
    pub async fn put<T: serde::Serialize + Sync + Entity>(&self, entity: &T) -> Result<()>
    {
        let serialized = ser::to_vec(entity).context(SerializeValueSnafu)?;
        self.0.put(entity.key().as_bytes(), serialized.as_ref()).await.context(DatabaseSnafu)
    }

    pub async fn range_iterator<K, T>(&self, range: T) -> Result<DbIterator<'_>>
    where
        K: AsRef<[u8]>,
        T: RangeBounds<K>,
    {
        self.0.scan(RangeAsRef!(range)).await.context(DatabaseSnafu)
    }

    async fn items_from_range<K, R, T: for<'de> serde::de::Deserialize<'de> + Sync + Entity>(&self, range: R) -> Vec<T> 
    where
        K: AsRef<[u8]>,
        R: RangeBounds<K>,
    {
        match self.range_iterator(range).await {
            Ok(mut iter) => {
                let mut items: Vec<T> = vec![];
                while let Ok(Some(item)) = iter.next().await {
                    items.push(
                        de::from_slice(&item.value).context(DeserializeValueSnafu).unwrap()
                    );
                }
                items
            },
            Err(e) => { println!("Error: {}", e); vec![] }
        }
    }

    pub async fn _get_page<T: for<'de> serde::de::Deserialize<'de> + Entity>(&self, key: Option<String>, count: u16) -> Result<Vec<T>>{
        let mut iter = match key {
            Some(k) => {
                let range_key = Bytes::from(k);
                self.0.scan(range_key..).await
            }
            _ => self.0.scan(..).await,
        }.unwrap();    // TODO: handle error

        let mut items: Vec<T> = vec![];
        while let Ok(Some(item)) = iter.next().await {
            items.push(
                de::from_slice(&item.value).context(DeserializeValueSnafu)?
            );
            if items.len() >= count.into() { break };
        }

        Ok(items)
    }

    pub async fn get_page(&self, key: Option<String>, count: u16) -> Result<Vec<HistoryItem>> {
        // using wrapper to get rid from template un result type
        self._get_page(key, count).await
    }

    /// Retrieves a value from the database by its key.
    ///
    /// # Errors
    ///
    /// Returns a `DbError` if the underlying database operation fails.
    /// Returns a `DeserializeError` if the value cannot be deserialized from JSON.
    pub async fn get<T: for<'de> serde::de::Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>> {
        let value: Option<bytes::Bytes> =
            self.0.get(key.as_bytes()).await.context(KeyGetSnafu {
                key: key.to_string(),
            })?;
        value.map_or_else(
            || Ok(None),
            |bytes| de::from_slice(&bytes).context(DeserializeValueSnafu), //.map_err(|e| Error::Deserialize { source: e}),
        )
    }
}

#[async_trait]
pub trait Repository {
    type Entity: Entity + Serialize + DeserializeOwned + Send + Sync;

    fn db(&self) -> &DbWrapper;

    async fn _create(&self, entity: &Self::Entity) -> Result<()> {
        let key = format!("{}.{}", Self::prefix(), entity.id());
        self.db()._put(&key, &entity).await?;
        Ok(())
    }

    async fn _get(&self, id: Uuid) -> Result<Self::Entity> {
        let key = format!("{}.{}", Self::prefix(), id);
        let entity = self.db().get(&key).await?;
        let entity = entity.ok_or(Error::KeyNotFound)?;
        Ok(entity)
    }

    async fn _delete(&self, id: Uuid) -> Result<()> {
        let key = format!("{}.{}", Self::prefix(), id);

        self.db().delete(&key).await?;
        Ok(())
    }   

    fn prefix() -> &'static str;
    fn collection_key() -> &'static str;
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb::{config::DbOptions};
    use slatedb::db::Db as SlateDb;
    use std::sync::Arc;
    use chrono::{DateTime, TimeZone, Utc};
    use tokio;
    use bytes::Bytes;
    use std::time::SystemTime;
    use futures::future::join_all;

    // #[async_trait]
    // pub trait QueryHistoryRepository: Send + Sync {
        // async fn create(&self, params: &HistoryItem) -> QueryHistoryResult<()>;
        // async fn get(&self, id: Uuid) -> QueryHistoryResult<HistoryItem>;
        // async fn delete(&self, id: Uuid) -> QueryHistoryResult<()>;
        // async fn scan(&self, start_key: Bytes, end_key:Bytes) -> QueryHistoryResult<DbIterator>;
        // async fn scan2<K, T>(&self, range: T) -> QueryHistoryResult<DbIterator>
        // where
        //     K: AsRef<[u8]>,
        //     T: RangeBounds<K>;
    // }

    // pub struct QueryHistoryRepositoryDb {
    //     db: Arc<DbWrapper>,
    // }

    // impl QueryHistoryRepositoryDb {
    //     pub const fn new(db: Arc<DbWrapper>) -> Self {
    //         Self { db }
    //     }
    // }

    // impl Repository for QueryHistoryRepositoryDb {
    //     type Entity = HistoryItem;

    //     fn db(&self) -> &DbWrapper {
    //         &self.db
    //     }

    //     fn prefix() -> &'static str {
    //         "qh"
    //     }

    //     fn collection_key() ->  &'static str {
    //         "qh.items"
    //     }
    // }

    // #[async_trait]
    // impl QueryHistoryRepository for QueryHistoryRepositoryDb {
        // async fn create(&self, entity: &HistoryItem) -> QueryHistoryResult<()> {
        //     self.db._put(entity.key().as_str(), &entity).await?;
        //     Ok(())
        // }

        // async fn get(&self, id: Uuid) -> QueryHistoryResult<HistoryItem> {
        //     Repository::_get(self, id).await.map_err(Into::into)
        // }

        // async fn delete(&self, id: Uuid) -> QueryHistoryResult<()> {
        //     Repository::_delete(self, id).await.map_err(Into::into)
        // }

        // async fn scan(&self, start_key: Bytes, end_key:Bytes) -> QueryHistoryResult<DbIterator> {
        //     Ok(self.db.0.scan(&start_key..=&end_key).await.context(ScanSnafu {
        //         start_key: String::from_utf8(start_key.to_vec()).unwrap_or("Can't convert start_key".to_string()),
        //         end_key: String::from_utf8(end_key.to_vec()).unwrap_or("Can't convert start_key".to_string()),
        //     })?)
        // }
        
    //}

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
        // let repo = QueryHistoryRepositoryDb::new(db.clone());
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

    async fn get_history_items(db: &Arc<DbWrapper>, cursor: Option<String>, page_size: u16) -> Vec<HistoryItem> {
        println!("fetch {} history items from cursor: {:?}", page_size, cursor);
        if let Ok(page_items) = db._get_page(cursor, page_size).await {
            // page_items.iter().for_each(|x| println!("{:?}", x));
            println!("Fetched count: {:?}\n", page_items.len());
            page_items
        } else {
            vec![]
        }
    }

    async fn get_some_items(db: &Arc<DbWrapper>, cursor: Option<String>, page_size: u16) -> Vec<SomeItem> {
        println!("fetch {} some items from cursor: {:?}", page_size, cursor);
        if let Ok(page_items) = db._get_page(cursor, page_size).await {
            // page_items.iter().for_each(|x| println!("{:?}", x));
            println!("Fetched count: {:?}\n", page_items.len());
            page_items
        } else {
            vec![]
        }
    }

    // async fn assert_check_range<K, R, T: for<'de> serde::de::Deserialize<'de> + Sync + Entity>(
    //     db: &DbWrapper,
    //     range: R,
    //     items: Vec<T>,
    // )
    // where
    //     K: AsRef<[u8]>,
    //     R: RangeBounds<K>,    
    // {
    //     let retrieved_items: Vec<T> = db.items_from_range(range).await;
    //     assert_eq!(items.len(), retrieved_items.len());
    //     assert_eq!(
    //         items.last().unwrap().key(),
    //         retrieved_items.last().unwrap().key(),
    //     );
    // }

    fn assert_check_items<T: serde::Serialize + Sync + Entity>(created_items: Vec<T>, retrieved_items: Vec<T>) {
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
    async fn test_slatedb_history_range_somes_range() {
        let (db, created_history_items) = create_populate_new_db().await;
        let created_some_items = populate_some_items(&db).await;

        let created = created_history_items;
        let range = created.first().unwrap().key()..=created.last().unwrap().key();
        let retrieved: Vec<HistoryItem> = db.items_from_range(range).await;
        assert_check_items(created, retrieved);

        let created = created_some_items;
        let range = created.first().unwrap().key()..=created.last().unwrap().key();
        let retrieved: Vec<SomeItem> = db.items_from_range(range).await;
        assert_check_items(created, retrieved);
    }


    #[tokio::test]
    async fn test_slatedb_single_key_prefix_unbound_range_items() {
        let (db, created) = create_populate_new_db().await;

        let range = ..;
        let retrieved: Vec<HistoryItem> = db.items_from_range(Range!(range)).await;
        assert_check_items(created, retrieved);
    }

    #[tokio::test]
    async fn test_slatedb_scan_full_range() {
        let (db, created_items) = create_populate_new_db().await;
        let retrieved_items = get_history_items(&db, None, 1000).await;
        assert_eq!(retrieved_items.len(), 100);
        assert_eq!(created_items[0].key(), retrieved_items[0].key());

        let some_items = get_some_items(&db, None, 2).await;
        assert_eq!(some_items.len(), 2);
    }

    #[tokio::test]
    async fn test_slatedb_scan_starting_non_existing_key_not_in_range() {
        let (db, created_items) = create_populate_new_db().await;
        let cursor = HistoryItem::key_from_time(
            DateTime::from_timestamp(
                Utc.with_ymd_and_hms(2019, 1, 5, 1, 0, 0).unwrap().timestamp(), 0
            ).unwrap()
        );
        let retrieved_items = get_history_items(&db, Some(cursor), 1).await;
        assert_eq!(retrieved_items.len(), 1);
        // retrieved first key in range, which is the closest one
        assert_eq!(created_items[0].key(), retrieved_items[0].key());
    }

    #[tokio::test]
    async fn test_slatedb_scan_existing_key_in_range() {
        let (db, created_items) = create_populate_new_db().await;
        let skip_first_n_items = 5;
        let cursor = created_items[skip_first_n_items].key();
        let retrieved_items = get_history_items(&db, Some(cursor), 1000).await;
        assert_eq!(retrieved_items.len(), created_items.len()-skip_first_n_items);
        assert_eq!(created_items.iter().last().unwrap().key(), retrieved_items.iter().last().unwrap().key());
    }

    #[tokio::test]
    async fn test_slatedb_scan_non_existing_key_in_range() {
        let (db, created_items) = create_populate_new_db().await;
        let cursor = HistoryItem::key_from_time(
            DateTime::from_timestamp(
                Utc.with_ymd_and_hms(2020, 1, 5, 1, 0, 0).unwrap().timestamp(), 0
            ).unwrap()
        );
        let retrieved_items = get_history_items(&db, Some(cursor), 10).await;
        assert_eq!(retrieved_items.len(), 10);
        assert_eq!(created_items[5].key(), retrieved_items[0].key());
    }
}
