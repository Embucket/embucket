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

use std::future::Future;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Range};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt, TryStream};
use serde_json::de;
use slatedb::db::Db as SlateDb;
use slatedb::db_iter::DbIterator as SlateDbIterator;
use snafu::prelude::*;
use crate::{DeserializeValueSnafu, ScanFailedSnafu, Result, Error};
use crate::Error::Database;

#[async_trait]
pub trait ScanIterator {
    type Next: Send + for<'de> serde::de::Deserialize<'de>;
    type Collectable;
    async fn next(&mut self) -> Result<Option<Self::Next>>;
    async fn collect(self) -> Result<Self::Collectable>;
}

pub trait Map: ScanIterator {
    type Iterator: Self;
    type Item: Send + for<'de> serde::de::Deserialize<'de>;
    type Transformed;
    type Mapper: FnMut(&Self::Item) -> Self::Transformed;
    fn map(self, f: Self::Mapper) -> MapScanIterator<Self::Iterator, Self::Item, Self::Transformed, Self::Mapper>;
}

pub trait Filter: ScanIterator {
    type Iterator: Self;
    type Item: Send + for<'de> serde::de::Deserialize<'de>;
    type Predicate: FnMut(&Self::Item) -> bool;
    fn filter(self, f: Self::Predicate) -> FilterScanIterator<Self::Iterator, Self::Item, Self::Predicate>;
}

pub struct VecScanIteratorBuilder<'a> {
    db: Arc<SlateDb>,
    key: &'a str,
    //From where to start the scan range for SlateDB
    // ex: if we ended on "tested2", the cursor would be "tested2"
    // and inside the `fn list_objects` in utils crate the start of the range would be "tested2\x00"
    // ("\x00" is the smallest ASCII char to find anything after the "tested2" excluding it)
    // and the whole range would be `tested2\x00..\x7F
    // (`\x7F` is the largest ASCII char to find anything before it)
    // if there are 4 tables `tested1..tested4` which would yield us "tested3" and "tested4" including other names if any exist
    cursor: Option<String>,
    limit: Option<usize>,
    //Search string, from where (and to where in lexicographical sort order) to do the search
    // ex: if we want to find all the test tables it could be "tes" (if there are 4 tables `tested1..tested4`)
    // the range would be `tes..tes\x7F` tables
    // (`\x7F` is the largest ASCII char to find anything before it)
    // if we however had the cursor from cursor comment (line 21)
    // we could also go from `tested2\x00..tes\x7F` which would yield us "tested3" and "tested4" only excluding other names if any exist
    token: Option<String>,
}

impl<'a> VecScanIteratorBuilder<'a> {
    pub fn cursor(self, cursor: String) -> Self {
        Self {
            cursor: Some(cursor),
            ..self
        }
    }
    pub fn token(self, token: String) -> Self {
        Self {
            token: Some(token),
            ..self
        }
    }
    fn limit(self, limit: usize) -> Self {
        Self {
            limit: Some(limit),
            ..self
        }
    }
    pub async fn iter(self) -> Result<VecScanIterator> {
        //We can look with respect to limit
        // from start to end (full scan),
        // from starts_with to start_with (search),
        // from cursor to end (looking not from the start)
        // and from cursor to prefix (search without starting at the start and looking to the end (no full scan))
        // more info in `list_config` file
        let start = self.token.clone().map_or_else(
            || format!("{key}/"),
            |search_prefix| format!("{key}/{search_prefix}"),
        );
        let start = self
            .cursor
            .map_or_else(|| start, |cursor| format!("{key}/{cursor}\x00"));
        let end = self.token.map_or_else(
            || format!("{key}/\x7F"),
            |search_prefix| format!("{key}/{search_prefix}\x7F"),
        );
        let range = Bytes::from(start)..Bytes::from(end);
        Ok(VecScanIterator {
            inner: self.db.scan(range).await.context(ScanFailedSnafu)?,
            limit: self.limit,
        })
    }
}

pub struct VecScanIterator<'a> {
    inner: SlateDbIterator<'a>,
    limit: Option<usize>,
}

impl<'a, T: Send + for<'de> serde::de::Deserialize<'de>> ScanIterator for VecScanIterator<'a> {
    type Next = T;

    type Collectable = Vec<T>;


    async fn next(&mut self) -> Result<Option<Self::Next>> {
        self
            .inner
            .next()
            .await
            .map_err(|e| Database { source: e })?
            .map_or_else(|| Ok(None), |key_value| Ok(Some(de::from_slice(&key_value.value).context(DeserializeValueSnafu)?)))
    }

    async fn collect(mut self) -> Result<Self::Collectable> {
        match self.limit {
            Some(limit) => {
                let mut objects: Self::Collectable = vec![];
                while let Ok(Some(value)) = self.next().await {
                    objects.push(value);
                    if objects.len() >= limit {
                        break;
                    }
                }
                Ok(objects)
            }
            None => {
                let mut objects: Self::Collectable = vec![];
                while let Ok(Some(value)) = self.next().await {
                    objects.push(value);
                }
                Ok(objects)
            }
        }
    }
}

impl<T: Send + for<'de> serde::de::Deserialize<'de>, F: FnMut(&T) -> bool> Filter for VecScanIterator<'_> {
    type Iterator = Self;
    type Item = Self::Next;
    type Predicate = F;

    fn filter(self, f: Self::Predicate) -> FilterScanIterator<Self::Iterator, Self::Item, Self::Predicate> {
        FilterScanIterator {
            iter: self,
            filter: f,
        }
    }
}

pub struct FilterScanIterator<I: ScanIterator, T: Send + for<'de> serde::de::Deserialize<'de>, F: FnMut(&T) -> bool> {
    iter: I,
    filter: F,
}

impl<I: ScanIterator, T: Send + for<'de> serde::de::Deserialize<'de>, F: FnMut(&T) -> bool> ScanIterator for FilterScanIterator<I, T, F>
{
    type Next = I::Next;
    type Collectable = I::Collectable;


    async fn next(&mut self) -> Result<Option<Self::Next>> {
        let value = self.iter.next().await?;
        match value {
            Some(value) => {
                if self.filter(&value) {
                    Ok(Some(value))
                } else {
                    self.next().await
                }
            }
            None => Ok(None)
        }
    }

    async fn collect(self) -> Result<Self::Collectable> {
        todo!()
    }
}

pub struct MapScanIterator<I: ScanIterator, T: Send + for<'de> serde::de::Deserialize<'de>, U, M: FnMut(&T) -> U> {
    iter: I,
    map: M,
}


