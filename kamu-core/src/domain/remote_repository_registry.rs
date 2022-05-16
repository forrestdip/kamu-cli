// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use super::*;
use opendatafabric::*;

use async_trait::async_trait;
use thiserror::Error;
use url::Url;

#[async_trait]
pub trait RemoteRepositoryRegistry: Send + Sync {
    fn get_all_repositories<'s>(&'s self) -> Box<dyn Iterator<Item = RepositoryName> + 's>;

    fn get_repository(
        &self,
        repo_name: &RepositoryName,
    ) -> Result<RepositoryAccessInfo, GetRepoError>;

    fn add_repository(&self, repo_name: &RepositoryName, url: Url) -> Result<(), AddRepoError>;

    fn delete_repository(&self, repo_name: &RepositoryName) -> Result<(), DeleteRepoError>;
}

////////////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
pub enum GetRepoError {
    #[error(transparent)]
    NotFound(
        #[from]
        #[backtrace]
        RepositoryNotFoundError,
    ),
    #[error(transparent)]
    Internal(
        #[from]
        #[backtrace]
        InternalError,
    ),
}

////////////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
pub enum AddRepoError {
    #[error(transparent)]
    AlreadyExists(
        #[from]
        #[backtrace]
        RepositoryAlreadyExistsError,
    ),
    #[error(transparent)]
    Internal(
        #[from]
        #[backtrace]
        InternalError,
    ),
}

////////////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
pub enum DeleteRepoError {
    #[error(transparent)]
    NotFound(
        #[from]
        #[backtrace]
        RepositoryNotFoundError,
    ),
    #[error(transparent)]
    Internal(
        #[from]
        #[backtrace]
        InternalError,
    ),
}

////////////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Clone, Eq, PartialEq, Debug)]
#[error("Repository {repo_name} does not exist")]
pub struct RepositoryNotFoundError {
    pub repo_name: RepositoryName,
}

////////////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Clone, Eq, PartialEq, Debug)]
#[error("Repository {repo_name} already exists")]
pub struct RepositoryAlreadyExistsError {
    pub repo_name: RepositoryName,
}
