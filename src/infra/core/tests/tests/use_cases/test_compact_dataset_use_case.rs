// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use std::assert_matches::assert_matches;
use std::sync::Arc;

use kamu::testing::MockDatasetActionAuthorizer;
use kamu::*;
use kamu_core::*;
use opendatafabric::{DatasetAlias, DatasetName};

use crate::tests::use_cases::*;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_compact_dataset_success() {
    let alias_foo = DatasetAlias::new(None, DatasetName::new_unchecked("foo"));

    let harness = CompactUseCaseHarness::new(
        MockDatasetActionAuthorizer::new().expect_check_write_dataset(&alias_foo, 1, true),
    );

    let foo = harness.create_root_dataset(&alias_foo).await;

    assert_matches!(
        harness.compact_dataset(ResolvedDataset::from(&foo)).await,
        Ok(_)
    );
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_compact_multiple_datasets_success() {
    let alias_foo = DatasetAlias::new(None, DatasetName::new_unchecked("foo"));
    let alias_bar = DatasetAlias::new(None, DatasetName::new_unchecked("bar"));

    let harness = CompactUseCaseHarness::new(
        MockDatasetActionAuthorizer::new()
            .expect_check_write_dataset(&alias_foo, 1, true)
            .expect_check_write_dataset(&alias_bar, 1, true),
    );

    let foo = harness.create_root_dataset(&alias_foo).await;
    let bar = harness.create_root_dataset(&alias_bar).await;

    let mut responses = harness
        .compact_datasets(vec![
            ResolvedDataset::from(&foo),
            ResolvedDataset::from(&bar),
        ])
        .await;

    assert_eq!(responses.len(), 2);
    let response_bar = responses.remove(1);
    let response_foo = responses.remove(0);

    assert_matches!(
        response_foo,
        CompactionResponse {
            result: Ok(_),
             dataset_ref
        } if dataset_ref == foo.dataset_handle.as_local_ref());
    assert_matches!(
        response_bar,
        CompactionResponse {
            result: Ok(_),
            dataset_ref
        } if dataset_ref == bar.dataset_handle.as_local_ref()
    );
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_compact_dataset_unauthorized() {
    let alias_foo = DatasetAlias::new(None, DatasetName::new_unchecked("foo"));

    let harness = CompactUseCaseHarness::new(
        MockDatasetActionAuthorizer::new().expect_check_write_dataset(&alias_foo, 1, false),
    );

    let foo = harness.create_root_dataset(&alias_foo).await;

    assert_matches!(
        harness.compact_dataset(ResolvedDataset::from(&foo)).await,
        Err(CompactionError::Access(_))
    );
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[tokio::test]
async fn test_compact_dataset_mixed_authorization_outcome() {
    let alias_foo = DatasetAlias::new(None, DatasetName::new_unchecked("foo"));
    let alias_bar = DatasetAlias::new(None, DatasetName::new_unchecked("bar"));

    let harness = CompactUseCaseHarness::new(
        MockDatasetActionAuthorizer::new()
            .expect_check_write_dataset(&alias_foo, 1, false)
            .expect_check_write_dataset(&alias_bar, 1, true),
    );

    let foo = harness.create_root_dataset(&alias_foo).await;
    let bar = harness.create_root_dataset(&alias_bar).await;

    let mut responses = harness
        .compact_datasets(vec![
            ResolvedDataset::from(&foo),
            ResolvedDataset::from(&bar),
        ])
        .await;

    assert_eq!(responses.len(), 2);
    let response_bar = responses.remove(1);
    let response_foo = responses.remove(0);

    assert_matches!(
        response_foo,
        CompactionResponse {
            result: Err(CompactionError::Access(_)),
             dataset_ref
        } if dataset_ref == foo.dataset_handle.as_local_ref());
    assert_matches!(
        response_bar,
        CompactionResponse {
            result: Ok(_),
            dataset_ref
        } if dataset_ref == bar.dataset_handle.as_local_ref()
    );
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[oop::extend(BaseUseCaseHarness, base_harness)]
struct CompactUseCaseHarness {
    base_harness: BaseUseCaseHarness,
    use_case: Arc<dyn CompactDatasetUseCase>,
}

impl CompactUseCaseHarness {
    fn new(mock_dataset_action_authorizer: MockDatasetActionAuthorizer) -> Self {
        let base_harness = BaseUseCaseHarness::new(
            BaseUseCaseHarnessOptions::new().with_authorizer(mock_dataset_action_authorizer),
        );

        let catalog = dill::CatalogBuilder::new_chained(base_harness.catalog())
            .add::<CompactDatasetUseCaseImpl>()
            .add::<CompactionPlannerImpl>()
            .add::<CompactionExecutorImpl>()
            .add::<ObjectStoreRegistryImpl>()
            .build();

        let use_case = catalog.get_one::<dyn CompactDatasetUseCase>().unwrap();

        Self {
            base_harness,
            use_case,
        }
    }

    async fn compact_dataset(
        &self,
        target: ResolvedDataset,
    ) -> Result<CompactionResult, CompactionError> {
        self.use_case
            .execute(target.get_handle(), CompactionOptions::default(), None)
            .await
    }

    async fn compact_datasets(&self, targets: Vec<ResolvedDataset>) -> Vec<CompactionResponse> {
        let handles: Vec<_> = targets
            .into_iter()
            .map(ResolvedDataset::take_handle)
            .collect();

        self.use_case
            .execute_multi(handles, CompactionOptions::default(), None)
            .await
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
