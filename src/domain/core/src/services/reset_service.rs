// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use internal_error::InternalError;
use opendatafabric::*;
use thiserror::Error;

use crate::entities::SetRefError;
use crate::*;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[async_trait::async_trait]
pub trait ResetService: Send + Sync {
    async fn reset_dataset(
        &self,
        target: ResolvedDataset,
        block_hash: Option<&Multihash>,
        old_head_maybe: Option<&Multihash>,
    ) -> Result<Multihash, ResetError>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Errors
////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Error)]
pub enum ResetError {
    #[error(transparent)]
    DatasetNotFound(
        #[from]
        #[backtrace]
        DatasetNotFoundError,
    ),
    #[error(transparent)]
    CASFailed(
        #[from]
        #[backtrace]
        RefCASError,
    ),
    #[error(transparent)]
    BlockNotFound(
        #[from]
        #[backtrace]
        BlockNotFoundError,
    ),
    #[error(transparent)]
    Access(
        #[from]
        #[backtrace]
        AccessError,
    ),
    #[error(transparent)]
    Internal(
        #[from]
        #[backtrace]
        InternalError,
    ),
    #[error(transparent)]
    OldHeadMismatch(
        #[from]
        #[backtrace]
        OldHeadMismatchError,
    ),
}

impl From<GetDatasetError> for ResetError {
    fn from(v: GetDatasetError) -> Self {
        match v {
            GetDatasetError::NotFound(e) => Self::DatasetNotFound(e),
            GetDatasetError::Internal(e) => Self::Internal(e),
        }
    }
}

impl From<auth::DatasetActionUnauthorizedError> for ResetError {
    fn from(v: auth::DatasetActionUnauthorizedError) -> Self {
        match v {
            auth::DatasetActionUnauthorizedError::Access(e) => Self::Access(e),
            auth::DatasetActionUnauthorizedError::Internal(e) => Self::Internal(e),
        }
    }
}

impl From<SetRefError> for ResetError {
    fn from(v: SetRefError) -> Self {
        match v {
            SetRefError::CASFailed(e) => Self::CASFailed(e),
            SetRefError::BlockNotFound(e) => Self::BlockNotFound(e),
            SetRefError::Access(e) => Self::Access(e),
            SetRefError::Internal(e) => Self::Internal(e),
        }
    }
}

#[derive(Error, Debug)]
#[error("Current head is {current_head} but expected head is {old_head}")]
pub struct OldHeadMismatchError {
    pub current_head: Multihash,
    pub old_head: Multihash,
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
