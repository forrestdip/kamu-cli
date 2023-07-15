// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use std::sync::Arc;

use domain::{CurrentAccountSubject, TEST_ACCOUNT_NAME};
use kamu::*;
use opendatafabric::AccountName;
use tempfile::TempDir;

use super::test_dataset_repository_shared;

/////////////////////////////////////////////////////////////////////////////////////////

fn local_fs_repo(tempdir: &TempDir, multi_tenant: bool) -> DatasetRepositoryLocalFs {
    let workspace_layout = WorkspaceLayout::create(tempdir.path(), false).unwrap();
    DatasetRepositoryLocalFs::new(
        workspace_layout.datasets_dir.clone(),
        Arc::new(CurrentAccountSubject::new_test()),
        multi_tenant,
    )
}

/////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_create_dataset() {
    let tempdir = tempfile::tempdir().unwrap();
    let repo = local_fs_repo(&tempdir, false);

    test_dataset_repository_shared::test_create_dataset(&repo, None).await;
}

/////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_create_dataset_multi_tenant() {
    let tempdir = tempfile::tempdir().unwrap();
    let repo = local_fs_repo(&tempdir, true);

    test_dataset_repository_shared::test_create_dataset(
        &repo,
        Some(AccountName::new_unchecked(TEST_ACCOUNT_NAME)),
    )
    .await;
}

/////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_create_dataset_same_name_multiple_tenants() {
    let tempdir = tempfile::tempdir().unwrap();
    let repo = local_fs_repo(&tempdir, true);

    test_dataset_repository_shared::test_create_dataset_same_name_multiple_tenants(&repo).await;
}

/////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_create_dataset_from_snapshot() {
    let tempdir = tempfile::tempdir().unwrap();
    let repo = local_fs_repo(&tempdir, false);

    test_dataset_repository_shared::test_create_dataset_from_snapshot(&repo, None).await;
}

/////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_create_dataset_from_snapshot_multi_tenant() {
    let tempdir = tempfile::tempdir().unwrap();
    let repo = local_fs_repo(&tempdir, true);

    test_dataset_repository_shared::test_create_dataset_from_snapshot(
        &repo,
        Some(AccountName::new_unchecked(TEST_ACCOUNT_NAME)),
    )
    .await;
}

/////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_rename_dataset() {
    let tempdir = tempfile::tempdir().unwrap();
    let repo = local_fs_repo(&tempdir, false);

    test_dataset_repository_shared::test_rename_dataset(&repo, None).await;
}

/////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_rename_dataset_multi_tenant() {
    let tempdir = tempfile::tempdir().unwrap();
    let repo = local_fs_repo(&tempdir, true);

    test_dataset_repository_shared::test_rename_dataset(
        &repo,
        Some(AccountName::new_unchecked(TEST_ACCOUNT_NAME)),
    )
    .await;
}

/////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_rename_dataset_same_name_multiple_tenants() {
    let tempdir = tempfile::tempdir().unwrap();
    let repo = local_fs_repo(&tempdir, true);

    test_dataset_repository_shared::test_rename_dataset_same_name_multiple_tenants(&repo).await;
}

/////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_delete_dataset() {
    let tempdir = tempfile::tempdir().unwrap();
    let repo = local_fs_repo(&tempdir, false);

    test_dataset_repository_shared::test_delete_dataset(&repo, None).await;
}

/////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_delete_dataset_multi_tenant() {
    let tempdir = tempfile::tempdir().unwrap();
    let repo = local_fs_repo(&tempdir, true);

    test_dataset_repository_shared::test_delete_dataset(
        &repo,
        Some(AccountName::new_unchecked(TEST_ACCOUNT_NAME)),
    )
    .await;
}

/////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_iterate_datasets() {
    let tempdir = tempfile::tempdir().unwrap();
    let repo = local_fs_repo(&tempdir, false);

    test_dataset_repository_shared::test_iterate_datasets(&repo).await;
}

/////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_iterate_datasets_multi_tenant() {
    let tempdir = tempfile::tempdir().unwrap();
    let repo = local_fs_repo(&tempdir, true);

    test_dataset_repository_shared::test_iterate_datasets_multi_tenant(&repo).await;
}

/////////////////////////////////////////////////////////////////////////////////////////