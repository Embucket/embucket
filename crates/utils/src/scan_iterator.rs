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
    async fn next(&mut self) -> Result<Option<Self::Next>>;
    async fn collect(mut self) -> Result<Vec<Self::Next>> {
        let mut objects: Vec<Self::Next> = Vec::new();
        while let Ok(Some(value)) = self.next().await {
            objects.push(value);
        }
        Ok(objects)
    }
}

#[async_trait]
pub trait Map: ScanIterator {
    type Iterator: ScanIterator;
    type Item: Send + for<'de> serde::de::Deserialize<'de>;
    type Transformed;
    type Mapper: FnMut(&Self::Item) -> Self::Transformed;
    fn map(self, f: Self::Mapper) -> MapScanIterator<Self::Iterator, Self::Item, Self::Transformed, Self::Mapper>;
}

#[async_trait]
pub trait Filter: ScanIterator {
    type Iterator: ScanIterator;
    type Item: Send + for<'de> serde::de::Deserialize<'de>;
    type Predicate: FnMut(&Self::Item) -> bool;
    fn filter(self, f: Self::Predicate) -> FilterScanIterator<Self::Iterator, Self::Item, Self::Predicate>;
}

pub struct VecScanIteratorBuilder<'a, T: Send + for<'de> serde::de::Deserialize<'de>> {
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
    marker: PhantomData<T>,
}

impl<'a, T: Send + for<'de> serde::de::Deserialize<'de>> VecScanIteratorBuilder<'a, T> {
    pub fn cursor(self, cursor: Option<String>) -> Self {
        Self {
            cursor: cursor,
            ..self
        }
    }
    pub fn token(self, token: Option<String>) -> Self {
        Self {
            token: token,
            ..self
        }
    }
    pub fn limit(self, limit: Option<usize>) -> Self {
        Self {
            limit: limit,
            ..self
        }
    }
    pub async fn iter(self) -> Result<VecScanIterator<'a, T>> {
        //We can look with respect to limit
        // from start to end (full scan),
        // from starts_with to start_with (search),
        // from cursor to end (looking not from the start)
        // and from cursor to prefix (search without starting at the start and looking to the end (no full scan))
        // more info in `list_config` file
        let start = self.token.clone().map_or_else(
            || format!("{}/", self.key),
            |search_prefix| format!("{}/{search_prefix}", self.key),
        );
        let start = self
            .cursor
            .map_or_else(|| start, |cursor| format!("{}/{cursor}\x00", self.key));
        let end = self.token.map_or_else(
            || format!("{}/\x7F", self.key),
            |search_prefix| format!("{}/{search_prefix}\x7F", self.key),
        );
        let range = Bytes::from(start)..Bytes::from(end);
        Ok(VecScanIterator {
            inner: self.db.scan(range).await.context(ScanFailedSnafu)?,
            i: 0,
            limit: self.limit,
            marker: PhantomData,
        })
    }
}

pub struct VecScanIterator<'a, T: Send + for<'de> serde::de::Deserialize<'de>> {
    inner: SlateDbIterator<'a>,
    i: usize,
    limit: Option<usize>,
    marker: PhantomData<T>,
}

impl<'a, T: Send + for<'de> serde::de::Deserialize<'de>> VecScanIterator<'a, T> {
    pub fn builder(db: Arc<SlateDb>, key: &'a str) -> VecScanIteratorBuilder<'a, T> {
        VecScanIteratorBuilder {
            db,
            key,
            cursor: None,
            limit: None,
            token: None,
            marker: PhantomData,
        }
    }
}

#[async_trait]
impl<'a, T: Send + for<'de> serde::de::Deserialize<'de>> ScanIterator for VecScanIterator<'a, T> {
    type Next = T;

    async fn next(&mut self) -> Result<Option<Self::Next>> {
        //TODO: rewrite duplication
        if let Some(limit) = self.limit {
            if self.i >= limit {
                Ok(None)
            } else {
                self.i += 1;
                self
                    .inner
                    .next()
                    .await
                    .map_err(|e| Database { source: e })?
                    .map_or_else(|| Ok(None), |key_value| Ok(Some(de::from_slice(&key_value.value).context(DeserializeValueSnafu)?)))
            }
        } else {
            self.i += 1;
            self
                .inner
                .next()
                .await
                .map_err(|e| Database { source: e })?
                .map_or_else(|| Ok(None), |key_value| Ok(Some(de::from_slice(&key_value.value).context(DeserializeValueSnafu)?)))
        }
    }
}

#[async_trait]
impl<T: Send + for<'de> serde::de::Deserialize<'de>, F: FnMut(&T) -> bool> Filter for VecScanIterator<'_, T> {
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

#[async_trait]
impl<T: Send + for<'de> serde::de::Deserialize<'de>, U, M: FnMut(&T) -> U> Map for VecScanIterator<'_, T> {
    type Iterator = Self;
    type Item = Self::Next;
    type Transformed = U;
    type Mapper = M;

    fn map(self, f: Self::Mapper) -> MapScanIterator<Self::Iterator, Self::Item, Self::Transformed, Self::Mapper> {
        MapScanIterator {
            iter: self,
            map: f,
        }
    }
}

pub struct FilterScanIterator<I: ScanIterator, T: Send + for<'de> serde::de::Deserialize<'de>, F: FnMut(&T) -> bool> {
    iter: I,
    filter: F,
}

#[async_trait]
impl<I: ScanIterator, T: Send + for<'de> serde::de::Deserialize<'de>, F: FnMut(&T) -> bool> ScanIterator for FilterScanIterator<I, T, F>
{
    type Next = I::Next;

    async fn next(&mut self) -> Result<Option<Self::Next>> {
        Ok(self.iter.next().await?.map(async |value| {
            if self.filter(&value) {
            Ok(Some(value))
            } else {
                self.next().await
            }
        }))
    }
}

#[async_trait]
impl<I: ScanIterator, T: Send + for<'de> serde::de::Deserialize<'de>, F: FnMut(&T) -> bool, U, M: FnMut(&T) -> U> Map for FilterScanIterator<I, T, F> {
    type Iterator = Self;
    type Item = Self::Next;
    type Transformed = U;
    type Mapper = M;

    fn map(self, f: Self::Mapper) -> MapScanIterator<Self::Iterator, Self::Item, Self::Transformed, Self::Mapper> {
        MapScanIterator {
            iter: self,
            map: f,
        }
    }
}

pub struct MapScanIterator<I: ScanIterator, T: Send + for<'de> serde::de::Deserialize<'de>, U, M: FnMut(&T) -> U> {
    iter: I,
    map: M,
}

#[async_trait]
impl<I: ScanIterator, T: Send + for<'de> serde::de::Deserialize<'de>, U: std::marker::Send, M: FnMut(&T) -> U> ScanIterator for MapScanIterator<I, T, U, M>
{
    type Next = U;

    async fn next(&mut self) -> Result<Option<Self::Next>> {
        Ok(self.iter.next().await?.map(|value| self.map(&value)))
    }
}

#[async_trait]
impl<I: ScanIterator, T: Send + for<'de> serde::de::Deserialize<'de>, F: FnMut(&T) -> bool, U, M: FnMut(&T) -> U> Filter for MapScanIterator<I, T, U, M> {
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


