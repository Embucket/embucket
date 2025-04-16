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
pub trait ScanAsyncIterator {
    type Next;
    type Collectable;
    async fn next(&mut self) -> Result<Self::Next>;
    async fn collect(self) -> Result<Self::Collectable>;
}

pub struct ScanIteratorBuilder<'a, T> {
    db: Arc<SlateDb>,
    key: &'a str,
    cursor: Option<String>,
    token: Option<String>,
    limit: Option<usize>,
    phantom: PhantomData<T>,
}

pub struct ScanIterator<'a> {
    inner: SlateDbIterator<'a>,
}

impl<'a, T, C: Send + for<'de> serde::de::Deserialize<'de>> ScanAsyncIterator for ScanIterator<'a> {
    type Next = T;

    type Collectable = Vec<C>;
    async fn next(&mut self) -> Result<Self::Next> {
        self.inner.next().await.map_err(|e| Database { source: e })
    }

    async fn collect(self) -> Result<Self::Collectable> {
        todo!()
    }
}

impl<'a> ScanIterator<'a> {
    pub fn new(db: Arc<SlateDb>, key: &str) -> Self {
        Self {
            db,
            key,
            cursor: None,
            token: None,
            limit: None
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
    // pub fn filter<F>(self, f: F) -> Self
    // where
    //     F: FnMut(&T) -> bool {
    //     Self {
    //         filter: Some(f),
    //         ..self
    //     }
    // }
    // pub fn map<F, U>(self, f: F) -> Self
    // where
    //     F: FnMut(&T) -> U {
    //     Self {
    //         map: Some(f),
    //         ..self
    //     }
    // }
    //fn sort_by<F>(&mut self, f: F) -> Self {
    //
    // }
}

pub struct FilterScanIter<'a, I: ScanIterator, T: Send + for<'de> serde::de::Deserialize<'de>, F: FnMut(&T) -> bool> {
    iter: I,
    filter: F,
}

pub struct MapScanIter<'a, I: ScanIterator, T: Send + for<'de> serde::de::Deserialize<'de>, U, M: FnMut(&T) -> U> {
    iter: I,
    map: M,
}


