// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use async_trait::async_trait;
use tracing::debug;

use crate::domain::repos::named_object_repository::GetError;
use crate::domain::*;
use opendatafabric::serde::yaml::Manifest;
use opendatafabric::*;

/////////////////////////////////////////////////////////////////////////////////////////

pub struct DatasetImpl<MetaChain, DataRepo, CheckpointRepo, InfoRepo> {
    metadata_chain: MetaChain,
    data_repo: DataRepo,
    checkpoint_repo: CheckpointRepo,
    info_repo: InfoRepo,
}

/////////////////////////////////////////////////////////////////////////////////////////

impl<MetaChain, DataRepo, CheckpointRepo, InfoRepo>
    DatasetImpl<MetaChain, DataRepo, CheckpointRepo, InfoRepo>
where
    MetaChain: MetadataChain2 + Sync + Send,
    DataRepo: ObjectRepository + Sync + Send,
    CheckpointRepo: ObjectRepository + Sync + Send,
    InfoRepo: NamedObjectRepository + Sync + Send,
{
    pub fn new(
        metadata_chain: MetaChain,
        data_repo: DataRepo,
        checkpoint_repo: CheckpointRepo,
        info_repo: InfoRepo,
    ) -> Self {
        Self {
            metadata_chain,
            data_repo,
            checkpoint_repo,
            info_repo,
        }
    }

    async fn read_summary(&self) -> Result<Option<DatasetSummary>, GetSummaryError> {
        let data = match self.info_repo.get("summary").await {
            Ok(data) => data,
            Err(GetError::NotFound(_)) => return Ok(None),
            Err(GetError::Access(e)) => return Err(GetSummaryError::Access(e)),
            Err(GetError::Internal(e)) => return Err(GetSummaryError::Internal(e)),
        };

        let manifest: Manifest<DatasetSummary> = serde_yaml::from_slice(&data[..]).int_err()?;

        if manifest.kind != "DatasetSummary" {
            return Err(InvalidObjectKind {
                expected: "DatasetSummary".to_owned(),
                actual: manifest.kind,
            }
            .int_err()
            .into());
        }

        Ok(Some(manifest.content))
    }

    async fn write_summary(&self, summary: &DatasetSummary) -> Result<(), GetSummaryError> {
        let manifest = Manifest {
            kind: "DatasetSummary".to_owned(),
            version: 1,
            content: summary.clone(),
        };

        let data = serde_yaml::to_vec(&manifest).int_err()?;

        match self.info_repo.set("summary", &data).await {
            Ok(()) => Ok(()),
            Err(SetError::Access(e)) => Err(GetSummaryError::Access(e)),
            Err(SetError::Internal(e)) => Err(GetSummaryError::Internal(e)),
        }?;

        Ok(())
    }

    async fn update_summary(
        &self,
        prev: Option<DatasetSummary>,
    ) -> Result<Option<DatasetSummary>, GetSummaryError> {
        let current_head = match self.metadata_chain.get_ref(&BlockRef::Head).await {
            Ok(h) => h,
            Err(GetRefError::NotFound(_)) => return Ok(prev),
            Err(GetRefError::Access(e)) => return Err(GetSummaryError::Access(e)),
            Err(GetRefError::Internal(e)) => return Err(GetSummaryError::Internal(e)),
        };

        let last_seen = prev.as_ref().map(|s| &s.last_block_hash);
        if last_seen == Some(&current_head) {
            return Ok(prev);
        }

        debug!(?current_head, ?last_seen, "Updating dataset summary");

        use tokio_stream::StreamExt;
        let blocks: Vec<_> = self
            .metadata_chain
            .iter_blocks_interval(&current_head, last_seen)
            .collect::<Result<_, _>>()
            .await
            .int_err()?;

        let mut summary = prev.unwrap_or(DatasetSummary {
            id: DatasetID::from_pub_key_ed25519(b""), // Will be replaced
            kind: DatasetKind::Root,
            last_block_hash: current_head.clone(),
            dependencies: Vec::new(),
            last_pulled: None,
            num_records: 0,
            data_size: 0,
            checkpoints_size: 0,
        });

        for (hash, block) in blocks.into_iter().rev() {
            summary.last_block_hash = hash;

            match block.event {
                MetadataEvent::Seed(seed) => {
                    summary.id = seed.dataset_id;
                    summary.kind = seed.dataset_kind;
                }
                MetadataEvent::SetTransform(set_transform) => {
                    summary.dependencies = set_transform.inputs;
                }
                MetadataEvent::AddData(add_data) => {
                    summary.last_pulled = Some(block.system_time);

                    let iv = add_data.output_data.interval;
                    summary.num_records += (iv.end - iv.start + 1) as u64;

                    summary.data_size += add_data.output_data.size as u64;

                    if let Some(checkpoint) = add_data.output_checkpoint {
                        summary.checkpoints_size += checkpoint.size as u64;
                    }
                }
                MetadataEvent::ExecuteQuery(execute_query) => {
                    summary.last_pulled = Some(block.system_time);

                    if let Some(output_data) = execute_query.output_data {
                        let iv = output_data.interval;
                        summary.num_records += (iv.end - iv.start + 1) as u64;

                        summary.data_size += output_data.size as u64;
                    }

                    if let Some(checkpoint) = execute_query.output_checkpoint {
                        summary.checkpoints_size += checkpoint.size as u64;
                    }
                }
                MetadataEvent::SetAttachments(_)
                | MetadataEvent::SetInfo(_)
                | MetadataEvent::SetLicense(_)
                | MetadataEvent::SetWatermark(_)
                | MetadataEvent::SetVocab(_)
                | MetadataEvent::SetPollingSource(_) => (),
            }
        }

        self.write_summary(&summary).await?;

        Ok(Some(summary))
    }
}

/////////////////////////////////////////////////////////////////////////////////////////

#[async_trait]
impl<MetaChain, DataRepo, CheckpointRepo, InfoRepo> Dataset
    for DatasetImpl<MetaChain, DataRepo, CheckpointRepo, InfoRepo>
where
    MetaChain: MetadataChain2 + Sync + Send,
    DataRepo: ObjectRepository + Sync + Send,
    CheckpointRepo: ObjectRepository + Sync + Send,
    InfoRepo: NamedObjectRepository + Sync + Send,
{
    async fn get_summary(&self, opts: SummaryOptions) -> Result<DatasetSummary, GetSummaryError> {
        let summary = self.read_summary().await?;

        let summary = if opts.update_if_stale {
            self.update_summary(summary).await?
        } else {
            summary
        };

        summary.ok_or_else(|| GetSummaryError::EmptyDataset)
    }

    fn as_metadata_chain(&self) -> &dyn MetadataChain2 {
        &self.metadata_chain
    }

    fn as_data_repo(&self) -> &dyn ObjectRepository {
        &self.data_repo
    }

    fn as_checkpoint_repo(&self) -> &dyn ObjectRepository {
        &self.checkpoint_repo
    }
}