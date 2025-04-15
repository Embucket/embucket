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
use bytes::Bytes;
use futures::{Stream, StreamExt, TryStream};
use serde_json::de;
use slatedb::db::Db as SlateDb;
use slatedb::db_iter::DbIterator;
use snafu::prelude::*;
use crate::{DeserializeValueSnafu, ScanFailedSnafu, Result, Error};
use crate::Error::Database;

pub struct ScanIterator<'a, T: Send + for<'de> serde::de::Deserialize<'de>, F: FnMut(&T) -> bool, U, M: FnMut(&T) -> U> {
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
    //Search string, from where (and to where in lexicographical sort order) to do the search
    // ex: if we want to find all the test tables it could be "tes" (if there are 4 tables `tested1..tested4`)
    // the range would be `tes..tes\x7F` tables
    // (`\x7F` is the largest ASCII char to find anything before it)
    // if we however had the cursor from cursor comment (line 21)
    // we could also go from `tested2\x00..tes\x7F` which would yield us "tested3" and "tested4" only excluding other names if any exist
    token: Option<String>,
    limit: Option<usize>,
    filter: Option<F>,
    map: Option<M>,
    //Holding the type without the value
    marker: PhantomData<T>,
}

impl<'a, T: Send + for<'de> serde::de::Deserialize<'de>, F: FnMut(&T) -> bool, U, M: FnMut(&T) -> U> ScanIterator<'a, T, F, U, M> {
    pub fn new(db: Arc<SlateDb>, key: &str) -> Self {
        Self {
            db,
            key,
            cursor: None,
            token: None,
            limit: None,
            filter: None,
            map: None,
            marker: Default::default(),
        }
    }
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
    pub fn limit(self, limit: usize) -> Self {
        Self {
            limit: Some(limit),
            ..self
        }
    }
    pub fn filter<F>(self, f: F) -> Self
    where
        F: FnMut(&T) -> bool {
        Self {
            filter: Some(f),
            ..self
        }
    }
    pub fn map<F, U>(self, f: F) -> Self
    where
        F: FnMut(&T) -> U {
        Self {
            map: Some(f),
            ..self
        }
    }
    //fn sort_by<F>(&mut self, f: F) -> Self {
    //
    // }
    pub async fn collect(self) -> Result<Vec<T>> {
        //We can look with respect to limit
        // from start to end (full scan),
        // from starts_with to start_with (search),
        // from cursor to end (looking not from the start)
        // and from cursor to prefix (search without starting at the start and looking to the end (no full scan))
        let start = self.token.clone().map_or_else(
            || format!("{}/", self.key),
            |search_prefix| format!("{}/{search_prefix}", self.key),
        );
        let start = self
            .cursor
            .clone()
            .map_or_else(|| start, |cursor| format!("{}/{cursor}\x00", self.key));
        let end = self.token.clone().map_or_else(
            || format!("{}/\x7F", self.key),
            |search_prefix| format!("{}/{search_prefix}\x7F", self.key),
        );
        let limit = self.limit.unwrap_or(usize::MAX);
        let filter = self.filter.unwrap_or(|_| true);
        let map = self.map.unwrap_or(|x| x);

        let range = Bytes::from(start)..Bytes::from(end);
        let mut iter = self.db.scan(range).await.context(ScanFailedSnafu)?;

        let mut objects: Vec<T> = vec![];
        while let Ok(Some(value)) = iter.next().await {
            let value = de::from_slice(&value.value).context(DeserializeValueSnafu)?;
            if filter(&value) {
                objects.push(map(&value));
            }
            if objects.len() >= limit {
                break;
            }
        }
        Ok(objects)
    }
}


