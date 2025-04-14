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

use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use bytes::Bytes;
use futures::{FutureExt, Stream, StreamExt, TryStream};
use serde_json::de;
use slatedb::db::Db as SlateDb;
use slatedb::db_iter::DbIterator;
use slatedb::error::SlateDBError;
use snafu::prelude::*;
use crate::{DeserializeValueSnafu, ScanFailedSnafu, Result, Error};

pub struct ScanIterator<'a, T> {
    inner: DbIterator<'a>,
    marker: PhantomData<T>,
    _db: Arc<SlateDb>,
}

impl<'a, T: Send + for<'de> serde::de::Deserialize<'de>> ScanIterator<'a, T> {
    pub fn builder(db: Arc<SlateDb>, key: &str) -> ScanIteratorBuilder<T> {
        ScanIteratorBuilder {
            db,
            key,
            cursor: None,
            token: None,
            marker: PhantomData,
        }
    }
}

impl<T: Send + for<'de> serde::de::Deserialize<'de> + std::marker::Unpin> Stream for ScanIterator<'_, T> {
    type Item = Result<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut test = self.get_mut().inner.next();
        let mut test = Box::pin(&mut test);
        match test.as_mut().poll_unpin(cx) {
            Poll::Ready(Ok(Some(item))) => {
                let value = de::from_slice(&item.value).context(DeserializeValueSnafu)?;
                Poll::Ready(Some(Ok(value)))
            }
            Poll::Ready(Ok(None)) => Poll::Ready(None),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e.into()))),
            Poll::Pending => Poll::Pending,
        }
    }
}

//TODO: is there a way to make result top level without try_next?
// impl<T: Send + for<'de> serde::de::Deserialize<'de>> TryStream for ScanIterator<'_, T> {
//     type Ok = T;
//     type Error = Error;
//
//     fn try_poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<std::result::Result<Self::Ok, Self::Error>>> {
//         self.poll_next(cx)
//     }
// }

pub struct ScanIteratorBuilder<'a, T> {
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
    //Holding the type without the value
    marker: PhantomData<T>,
}

impl<'a, T: Send + for<'de> serde::de::Deserialize<'de>> ScanIteratorBuilder<'a, T> {
    pub fn cursor(self, cursor: Option<String>) -> Self {
        Self {
            db: self.db,
            key: self.key,
            cursor,
            token: self.token,
            marker: PhantomData,
        }
    }
    pub fn token(self, token: Option<String>) -> Self {
        Self {
            db: self.db,
            key: self.key,
            cursor: self.cursor,
            token,
            marker: PhantomData,
        }
    }
    pub async fn iter(&'a self) -> Result<ScanIterator<'a, T>> {
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
            .clone()
            .map_or_else(|| start, |cursor| format!("{}/{cursor}\x00", self.key));
        let end = self.token.clone().map_or_else(
            || format!("{}/\x7F", self.key),
            |search_prefix| format!("{}/{search_prefix}\x7F", self.key),
        );
        let range = Bytes::from(start)..Bytes::from(end);
        Ok(ScanIterator {
            inner: self.db.scan(range).await.context(ScanFailedSnafu)?,
            marker: PhantomData,
            _db: self.db.clone(),
        })
    }
}


