// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::de;
use serde_json::ser;
use slatedb::db::Db as SlateDb;
use slatedb::error::SlateDBError;
use snafu::prelude::*;
use uuid::Uuid;

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

pub struct Db(SlateDb);

impl Db {
    pub const fn new(db: SlateDb) -> Self {
        Self(db)
    }

    /// Closes the database connection.
    ///
    /// # Errors
    ///
    /// Returns a `DbError` if the underlying database operation fails.
    pub async fn close(&self) -> Result<()> {
        self.0.close().await.context(DatabaseSnafu)?;
        Ok(())
    }

    /// Deletes a key-value pair from the database.
    ///
    /// # Errors
    ///
    /// This function will return a `DbError` if the underlying database operation fails.
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.0.delete(key.as_bytes()).await;
        Ok(())
    }

    /// Stores a key-value pair in the database.
    ///
    /// # Errors
    ///
    /// Returns a `SerializeError` if the value cannot be serialized to JSON.
    /// Returns a `DbError` if the underlying database operation fails.
    pub async fn put<T: serde::Serialize + Sync>(&self, key: &str, value: &T) -> Result<()> {
        let serialized = ser::to_vec(value).context(SerializeValueSnafu)?;
        self.0.put(key.as_bytes(), serialized.as_ref()).await;
        Ok(())
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

    /// Retrieves a list of keys from the database.
    ///
    /// # Errors
    ///
    /// Returns a `DbError` if the underlying database operation fails.
    /// Returns a `DeserializeError` if the value cannot be deserialized from JSON.
    pub async fn keys(&self, key: &str) -> Result<Vec<String>> {
        let keys: Option<Vec<String>> = self.get(key).await?;
        Ok(keys.unwrap_or_default())
    }

    /// Appends a value to a list stored in the database.
    ///
    /// # Errors
    ///
    /// Returns a `DbError` if the database operations fail, or
    /// `SerializeError`/`DeserializeError` if the value cannot be serialized or deserialized.
    pub async fn append(&self, key: &str, value: String) -> Result<()> {
        self.modify(key, |all_keys: &mut Vec<String>| {
            if !all_keys.contains(&value) {
                all_keys.push(value.clone());
            }
        })
        .await?;
        Ok(())
    }

    /// Removes a value from a list stored in the database.
    ///
    /// # Errors
    ///
    /// Returns a `DbError` if the database operations fail, or
    /// `SerializeError`/`DeserializeError` if the value cannot be serialized or deserialized.
    pub async fn remove(&self, key: &str, value: &str) -> Result<()> {
        self.modify(key, |all_keys: &mut Vec<String>| {
            all_keys.retain(|key| *key != value);
        })
        .await?;
        Ok(())
    }

    /// Modifies a value in the database using the provided closure.
    ///
    /// # Errors
    ///
    /// Returns a `DbError` if the database operations fail, or
    /// `SerializeError`/`DeserializeError` if the value cannot be serialized or deserialized.
    pub async fn modify<T>(&self, key: &str, f: impl Fn(&mut T) + Send) -> Result<()>
    where
        T: serde::Serialize + DeserializeOwned + Default + Sync + Send,
    {
        let mut value: T = self.get(key).await?.unwrap_or_default();

        f(&mut value);

        self.put(key, &value).await?;

        Ok(())
    }
}

impl From<Error> for iceberg::Error {
    fn from(e: Error) -> Self {
        Self::new(iceberg::ErrorKind::Unexpected, e.to_string()).with_source(e)
    }
}

#[async_trait]
pub trait Entity {
    fn id(&self) -> Uuid;
}

#[async_trait]
pub trait Repository {
    type Entity: Entity + Serialize + DeserializeOwned + Send + Sync;

    fn db(&self) -> &Db;

    async fn _create(&self, entity: &Self::Entity) -> Result<()> {
        let key = format!("{}.{}", Self::prefix(), entity.id());
        self.db().put(&key, &entity).await?;
        self.db().append(Self::collection_key(), key).await?;
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
        self.db().remove(Self::collection_key(), &key).await?;
        Ok(())
    }

    async fn _list(&self) -> Result<Vec<Self::Entity>> {
        let keys = self.db().keys(Self::collection_key()).await?;
        let futures = keys
            .iter()
            .map(|key| self.db().get(key))
            .collect::<Vec<_>>();
        let results = futures::future::try_join_all(futures).await?;
        let entities = results.into_iter().flatten().collect::<Vec<Self::Entity>>();
        Ok(entities)
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
    use slatedb::{config::DbOptions, db_iter::DbIterator};
    use slatedb::db::Db as SlateDb;
    use std::{ops::RangeBounds, sync::Arc};
    use serde::{Deserialize, Serialize};
    use chrono::{DateTime, TimeZone, Utc};
    use tokio;
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};
    use tracing;
    use futures::future::join_all;

    pub type QueryHistoryResult<T> = Result<T>;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct HistoryItem {
        pub id: Uuid,
        pub query: String,
        pub start_time: DateTime<Utc>,
        pub end_time: DateTime<Utc>,
        pub status_code: u16,
    }

    impl HistoryItem {
        pub fn key(&self) -> String {
            self.start_time.timestamp().to_string()
        }
    }

    impl Entity for HistoryItem {
        fn id(&self) -> Uuid {
            self.id
        }
    }

    #[async_trait]
    pub trait QueryHistoryRepository: Send + Sync {
        async fn create(&self, params: &HistoryItem) -> QueryHistoryResult<()>;
        async fn get(&self, id: Uuid) -> QueryHistoryResult<HistoryItem>;
        async fn delete(&self, id: Uuid) -> QueryHistoryResult<()>;
        async fn list(&self) -> QueryHistoryResult<Vec<HistoryItem>>;
        async fn scan(&self, start_key: Bytes, end_key:Bytes) -> QueryHistoryResult<DbIterator>;
        // async fn scan2<K, T>(&self, range: T) -> QueryHistoryResult<DbIterator>
        // where
        //     K: AsRef<[u8]>,
        //     T: RangeBounds<K>;
    }

    pub struct QueryHistoryRepositoryDb {
        db: Arc<Db>,
    }

    impl QueryHistoryRepositoryDb {
        pub const fn new(db: Arc<Db>) -> Self {
            Self { db }
        }
    }

    impl Repository for QueryHistoryRepositoryDb {
        type Entity = HistoryItem;

        fn db(&self) -> &Db {
            &self.db
        }

        fn prefix() -> &'static str {
            "qh"
        }

        fn collection_key() ->  &'static str {
            "qh.items"
        }
    }

    #[async_trait]
    impl QueryHistoryRepository for QueryHistoryRepositoryDb {
        async fn create(&self, entity: &HistoryItem) -> QueryHistoryResult<()> {
            self.db.put(entity.key().as_str(), &entity).await?;
            Ok(())
        }

        async fn get(&self, id: Uuid) -> QueryHistoryResult<HistoryItem> {
            Repository::_get(self, id).await.map_err(Into::into)
        }

        async fn delete(&self, id: Uuid) -> QueryHistoryResult<()> {
            Repository::_delete(self, id).await.map_err(Into::into)
        }

        async fn list(&self) -> QueryHistoryResult<Vec<HistoryItem>> {
            Repository::_list(self).await.map_err(Into::into)
        }

        // async fn scan2<K, T>(&self, range: T) -> QueryHistoryResult<DbIterator>
        // where
        //     K: AsRef<[u8]>,
        //     T: RangeBounds<K>,
        // {
        //     let start = range
        //         .start_bound()
        //         .map(|b| Bytes::copy_from_slice(b.as_ref()));
        //     let end = range
        //         .end_bound()
        //         .map(|b| Bytes::copy_from_slice(b.as_ref()));
        //     // let range = (start, end);
                        
            
        // }

        async fn scan(&self, start_key: Bytes, end_key:Bytes) -> QueryHistoryResult<DbIterator> {
            Ok(self.db.0.scan(&start_key..=&end_key).await.context(ScanSnafu {
                start_key: String::from_utf8(start_key.to_vec()).unwrap_or("Can't convert start_key".to_string()),
                end_key: String::from_utf8(end_key.to_vec()).unwrap_or("Can't convert start_key".to_string()),
            })?)
        }
        
    }

    async fn create_slate_db() -> Db {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let options = DbOptions::default();
        Db::new(
            SlateDb::open_with_opts(Path::from("/tmp/test_kv_store"), options, object_store)
                .await
                .unwrap(),
        )
    }

    fn newHistoryItem(prev: Option<HistoryItem>) -> HistoryItem {
        match prev {
            Some(item) => {
                let end_ts = item.end_time.timestamp();
                HistoryItem {
                    id: Uuid::new_v4(),
                    query: String::from(format!("SELECT {}", item.start_time)),
                    start_time: DateTime::from_timestamp(end_ts + 1, 0).unwrap(),
                    end_time: DateTime::from_timestamp(end_ts + 2, 0).unwrap(),
                    status_code: 200,
                }
            }
            _ => {
                let start_ts = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap().timestamp();
                let end_ts = start_ts + 1;
                HistoryItem {
                    id: Uuid::new_v4(),
                    query: String::from(format!("SELECT {start_ts}")),
                    start_time: DateTime::from_timestamp(start_ts, 0).unwrap(),
                    end_time: DateTime::from_timestamp(end_ts, 0).unwrap(),
                    status_code: 200,
                }
            }
        }
    }

    #[tokio::test]
    async fn test_slatedb_scan() {
        let db = Arc::new(create_slate_db().await);
        let repo = QueryHistoryRepositoryDb::new(db.clone());
        let mut item: Option<HistoryItem> = None;
        
        let started = SystemTime::now();
        println!("Create QueryHistory items {:?}", SystemTime::now().duration_since(started));

        const COUNT: usize = 1000;
        let mut items: Vec<HistoryItem> = vec![];
        for n in 0..COUNT {
            item = Some(newHistoryItem(item));
            items.push(item.clone().unwrap());
        }
        println!("QueryHistory items created {:?}", SystemTime::now().duration_since(started));

        let write_count_in_blocking_mode = 1;
        let mut fut = Vec::new();
        for (i, item) in items.iter().enumerate() {
            if i < write_count_in_blocking_mode {
                let _res = repo.create(&item).await;
                if i+1 == write_count_in_blocking_mode {
                    println!(
                        "QueryHistory items count={} part-1 stored in blocking mode {:?}", 
                        write_count_in_blocking_mode, 
                        SystemTime::now().duration_since(started)
                    );
                }
            } else {
                fut.push(repo.create(&item))
            }
        }
        join_all(fut).await;
        println!(
            "Rest of QueryHistory items count={} stored in non blocking mode {:?}", 
            items.len()-write_count_in_blocking_mode,
            SystemTime::now().duration_since(started)
        );

        // repo.scan(start_key, end_key);
        let mut iter = repo.db.0.scan(..).await.unwrap();
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
        assert_eq!(i, COUNT);
        assert_eq!(Arc::as_ptr(&repo.db), Arc::as_ptr(&db));
        println!("QueryHistory items retrieved and checked {:?}", SystemTime::now().duration_since(started));

        let mut iter = repo.db.0.scan(Bytes::from(items[0].key())..).await.unwrap();
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
        assert_eq!(i, COUNT);
        assert_eq!(Arc::as_ptr(&repo.db), Arc::as_ptr(&db));
        println!("QueryHistory items retrieved and checked again {:?}", SystemTime::now().duration_since(started));

        // Create QueryHistory items Ok(40ns)
        // QueryHistory items created Ok(1.481968ms)
        // QueryHistory items count=1 part-1 stored in blocking mode Ok(100.554187ms)
        // Rest of QueryHistory items count=999 stored in non blocking mode Ok(237.127829ms)
        // QueryHistory items retrieved and checked Ok(256.275055ms)
        // QueryHistory items retrieved and checked again Ok(269.152919ms)

        assert_eq!(0,1);
    }
}