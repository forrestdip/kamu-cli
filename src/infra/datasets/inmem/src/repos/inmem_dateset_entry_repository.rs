// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use std::collections::HashMap;
use std::sync::Arc;

use dill::{component, interface, scope, Singleton};
use kamu_datasets::{
    DatasetEntry,
    DatasetEntryByNameNotFoundError,
    DatasetEntryNameCollisionError,
    DatasetEntryNotFoundError,
    DatasetEntryRepository,
    DeleteEntryDatasetError,
    GetDatasetEntriesByOwnerIdError,
    GetDatasetEntryByNameError,
    GetDatasetEntryError,
    SaveDatasetEntryError,
    SaveDatasetEntryErrorDuplicate,
    UpdateDatasetEntryNameError,
};
use opendatafabric::{AccountID, DatasetID, DatasetName};
use tokio::sync::RwLock;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
struct State {
    rows: HashMap<DatasetID, DatasetEntry>,
}

impl State {
    fn new() -> Self {
        Self {
            rows: HashMap::new(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct InMemoryDatasetEntryRepository {
    state: Arc<RwLock<State>>,
}

#[component(pub)]
#[interface(dyn DatasetEntryRepository)]
#[scope(Singleton)]
impl InMemoryDatasetEntryRepository {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::new())),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[async_trait::async_trait]
impl DatasetEntryRepository for InMemoryDatasetEntryRepository {
    async fn dataset_entries_count(&self) -> Result<usize, GetDatasetEntryError> {
        let readable_state = self.state.read().await;

        let dataset_entries_count = readable_state.rows.len();

        Ok(dataset_entries_count)
    }

    async fn get_dataset_entry(
        &self,
        dataset_id: &DatasetID,
    ) -> Result<DatasetEntry, GetDatasetEntryError> {
        let readable_state = self.state.read().await;

        let maybe_dataset_entry = readable_state.rows.get(dataset_id);

        let Some(dataset_entry) = maybe_dataset_entry else {
            return Err(DatasetEntryNotFoundError::new(dataset_id.clone()).into());
        };

        Ok(dataset_entry.clone())
    }

    async fn get_dataset_entry_by_name(
        &self,
        owner_id: &AccountID,
        name: &DatasetName,
    ) -> Result<DatasetEntry, GetDatasetEntryByNameError> {
        let readable_state = self.state.read().await;

        let maybe_dataset_entry = readable_state
            .rows
            .values()
            .find(|dataset| dataset.owner_id == *owner_id && dataset.name == *name);

        let Some(dataset_entry) = maybe_dataset_entry else {
            return Err(
                DatasetEntryByNameNotFoundError::new(owner_id.clone(), name.clone()).into(),
            );
        };

        Ok(dataset_entry.clone())
    }

    async fn get_dataset_entries_by_owner_id(
        &self,
        owner_id: &AccountID,
    ) -> Result<Vec<DatasetEntry>, GetDatasetEntriesByOwnerIdError> {
        let readable_state = self.state.read().await;

        let dataset_entries = readable_state
            .rows
            .values()
            .fold(vec![], |mut acc, dataset| {
                if dataset.owner_id == *owner_id {
                    acc.push(dataset.clone());
                }

                acc
            });

        Ok(dataset_entries)
    }

    async fn save_dataset_entry(
        &self,
        dataset_entry: &DatasetEntry,
    ) -> Result<(), SaveDatasetEntryError> {
        let mut writable_state = self.state.write().await;

        for row in writable_state.rows.values() {
            if row.id == dataset_entry.id {
                return Err(SaveDatasetEntryErrorDuplicate::new(dataset_entry.id.clone()).into());
            }

            if row.owner_id == dataset_entry.owner_id && row.name == dataset_entry.name {
                return Err(DatasetEntryNameCollisionError::new(dataset_entry.name.clone()).into());
            }
        }

        writable_state
            .rows
            .insert(dataset_entry.id.clone(), dataset_entry.clone());

        Ok(())
    }

    async fn update_dataset_entry_name(
        &self,
        dataset_id: &DatasetID,
        new_name: &DatasetName,
    ) -> Result<(), UpdateDatasetEntryNameError> {
        let mut writable_state = self.state.write().await;

        let maybe_dataset_entry = writable_state.rows.get(dataset_id);

        let Some(found_dataset_entry) = maybe_dataset_entry else {
            return Err(DatasetEntryNotFoundError::new(dataset_id.clone()).into());
        };

        let has_name_collision_detected = writable_state.rows.values().any(|dataset_entry| {
            dataset_entry.id != *dataset_id
                && dataset_entry.owner_id == found_dataset_entry.owner_id
                && dataset_entry.name == *new_name
        });

        if has_name_collision_detected {
            return Err(DatasetEntryNameCollisionError::new(new_name.clone()).into());
        }

        // To avoid frustrating the borrow checker, we have to do a second look-up.
        // Safety: We're already guaranteed that the entry will be present.
        let found_dataset_entry = writable_state.rows.get_mut(dataset_id).unwrap();

        found_dataset_entry.name = new_name.clone();

        Ok(())
    }

    async fn delete_dataset_entry(
        &self,
        dataset_id: &DatasetID,
    ) -> Result<(), DeleteEntryDatasetError> {
        let mut writable_state = self.state.write().await;

        let not_found = writable_state.rows.remove(dataset_id).is_none();

        if not_found {
            return Err(DatasetEntryNotFoundError::new(dataset_id.clone()).into());
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
