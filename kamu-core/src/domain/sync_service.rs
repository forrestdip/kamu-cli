// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use super::{BoxedError, CreateDatasetError, GetDatasetError, InternalError};
use opendatafabric::*;

use std::sync::Arc;
use thiserror::Error;
use url::Url;

///////////////////////////////////////////////////////////////////////////////
// Service
///////////////////////////////////////////////////////////////////////////////

#[async_trait::async_trait(?Send)]
pub trait SyncService: Send + Sync {
    async fn sync(
        &self,
        src: &DatasetRefAny,
        dst: &DatasetRefAny,
        opts: SyncOptions,
        listener: Option<Arc<dyn SyncListener>>,
    ) -> Result<SyncResult, SyncError>;

    async fn sync_multi(
        &self,
        src_dst: &mut dyn Iterator<Item = (DatasetRefAny, DatasetRefAny)>,
        opts: SyncOptions,
        listener: Option<Arc<dyn SyncMultiListener>>,
    ) -> Vec<SyncResultMulti>;
}

#[derive(Debug, Clone)]
pub struct SyncOptions {
    /// Whether the source of data can be assumed non-malicious to skip hash sum and other expensive checks.
    /// Defaults to `true` when the source is local workspace.
    pub trust_source: Option<bool>,

    /// Whether destination dataset should be created if it does not exist
    pub create_if_not_exist: bool,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            trust_source: None,
            create_if_not_exist: true,
        }
    }
}

#[derive(Debug)]
pub enum SyncResult {
    UpToDate,
    Updated {
        old_head: Option<Multihash>,
        new_head: Multihash,
        num_blocks: usize,
    },
}

#[derive(Debug)]
pub struct SyncResultMulti {
    pub src: DatasetRefAny,
    pub dst: DatasetRefAny,
    pub result: Result<SyncResult, SyncError>,
}

///////////////////////////////////////////////////////////////////////////////
// Listener
///////////////////////////////////////////////////////////////////////////////

pub trait SyncListener: Send {
    fn begin(&self) {}
    fn success(&self, _result: &SyncResult) {}
    fn error(&self, _error: &SyncError) {}
}

pub struct NullSyncListener;
impl SyncListener for NullSyncListener {}

pub trait SyncMultiListener {
    fn begin_sync(
        &self,
        _src: &DatasetRefAny,
        _dst: &DatasetRefAny,
    ) -> Option<Arc<dyn SyncListener>> {
        None
    }
}

pub struct NullSyncMultiListener;
impl SyncMultiListener for NullSyncMultiListener {}

///////////////////////////////////////////////////////////////////////////////
// Errors
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Error)]
pub enum SyncError {
    #[error(transparent)]
    DatasetNotFound(#[from] DatasetNotFoundError),
    #[error(transparent)]
    CreateDatasetFailed(#[from] CreateDatasetError),
    #[error(transparent)]
    UnsupportedProtocol(#[from] UnsupportedProtocolError),
    #[error("Repository {repo_name} does not exist")]
    RepositoryDoesNotExist { repo_name: RepositoryName },
    // TODO: Report divergence type (e.g. remote is ahead of local)
    //#[error("Local dataset ({local_head}) and remote ({remote_head}) have diverged (remote is ahead by {uncommon_blocks_in_remote} blocks, local is ahead by {uncommon_blocks_in_local})")]
    #[error(transparent)]
    DatasetsDiverged(#[from] DatasetsDivergedError),
    #[error(transparent)]
    Corrupted(#[from] CorruptedSourceError),
    #[error("dataset was updated concurrently")]
    UpdatedConcurrently(#[source] BoxedError),
    //#[error("{0}")]
    //Unauthorized(#[source] BoxedError),
    //#[error("{0}")]
    //ReadOnly(#[source] BoxedError),
    //#[error("{0}")]
    //ProtocolError(#[source] BoxedError),
    #[error(transparent)]
    Internal(#[from] InternalError),
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Error, Clone, Eq, PartialEq, Debug)]
#[error("dataset {dataset_ref} not found")]
pub struct DatasetNotFoundError {
    pub dataset_ref: DatasetRefAny,
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Error, Clone, Eq, PartialEq, Debug)]
#[error("source and destination datasets have diverged, source head is {src_head}, destination head is {dst_head}")]
pub struct DatasetsDivergedError {
    pub src_head: Multihash,
    pub dst_head: Multihash,
    //uncommon_blocks_in_local: usize,
    //uncommon_blocks_in_remote: usize,
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
#[error("repository appears to have corrupted data: {message}")]
pub struct CorruptedSourceError {
    pub message: String,
    #[source]
    pub source: Option<BoxedError>,
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
pub struct UnsupportedProtocolError {
    pub url: Url,
}
impl std::fmt::Display for UnsupportedProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "usupported protocol {} when accessing {}",
            self.url.scheme(),
            self.url
        )
    }
}

///////////////////////////////////////////////////////////////////////////////

impl From<GetDatasetError> for SyncError {
    fn from(v: GetDatasetError) -> Self {
        match v {
            GetDatasetError::NotFound(e) => Self::DatasetNotFound(DatasetNotFoundError {
                dataset_ref: e.dataset_ref.into(),
            }),
            GetDatasetError::Internal(e) => Self::Internal(e),
        }
    }
}
