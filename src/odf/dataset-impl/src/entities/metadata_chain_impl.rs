// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use internal_error::ErrorIntoInternal;
use odf_dataset::*;
use odf_metadata::*;
use odf_storage::*;

use super::metadata_chain_validators::*;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

macro_rules! invalid_event {
    ($e:expr, $msg:expr $(,)?) => {
        return Err(AppendValidationError::InvalidEvent(InvalidEventError::new(
            $e, $msg,
        )))
    };
}

pub(crate) use invalid_event;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct MetadataChainImpl<MetaBlockRepo, RefRepo> {
    meta_block_repo: MetaBlockRepo,
    ref_repo: RefRepo,
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

impl<MetaBlockRepo, RefRepo> MetadataChainImpl<MetaBlockRepo, RefRepo>
where
    MetaBlockRepo: MetadataBlockRepository + Sync + Send,
    RefRepo: ReferenceRepository + Sync + Send,
{
    pub fn new(meta_block_repo: MetaBlockRepo, ref_repo: RefRepo) -> Self {
        Self {
            meta_block_repo,
            ref_repo,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[async_trait::async_trait]
impl<MetaBlockRepo, RefRepo> MetadataChain for MetadataChainImpl<MetaBlockRepo, RefRepo>
where
    MetaBlockRepo: MetadataBlockRepository + Sync + Send,
    RefRepo: ReferenceRepository + Sync + Send,
{
    async fn resolve_ref(&self, r: &BlockRef) -> Result<Multihash, GetRefError> {
        self.ref_repo.get(r.as_str()).await
    }

    async fn contains_block(&self, hash: &Multihash) -> Result<bool, ContainsBlockError> {
        self.meta_block_repo.contains_block(hash).await
    }

    async fn get_block_size(&self, hash: &Multihash) -> Result<u64, GetBlockDataError> {
        self.meta_block_repo.get_block_size(hash).await
    }

    async fn get_block_bytes(&self, hash: &Multihash) -> Result<bytes::Bytes, GetBlockDataError> {
        self.meta_block_repo.get_block_bytes(hash).await
    }

    async fn get_block(&self, hash: &Multihash) -> Result<MetadataBlock, GetBlockError> {
        self.meta_block_repo
            .get_block(hash)
            .await
            .map_err(Into::into)
    }

    async fn set_ref<'a>(
        &'a self,
        r: &BlockRef,
        hash: &Multihash,
        opts: SetRefOpts<'a>,
    ) -> Result<(), SetChainRefError> {
        if opts.validate_block_present {
            match self.meta_block_repo.contains_block(hash).await {
                Ok(true) => Ok(()),
                Ok(false) => Err(SetChainRefError::BlockNotFound(BlockNotFoundError {
                    hash: hash.clone(),
                })),
                Err(ContainsBlockError::Access(e)) => Err(SetChainRefError::Access(e)),
                Err(ContainsBlockError::Internal(e)) => Err(SetChainRefError::Internal(e)),
            }?;
        }

        // TODO: CONCURRENCY: Implement true CAS
        if let Some(prev_expected) = opts.check_ref_is {
            let prev_actual = match self.ref_repo.get(r.as_str()).await {
                Ok(r) => Ok(Some(r)),
                Err(GetRefError::NotFound(_)) => Ok(None),
                Err(GetRefError::Access(e)) => Err(SetChainRefError::Access(e)),
                Err(GetRefError::Internal(e)) => Err(SetChainRefError::Internal(e)),
            }?;
            if prev_expected != prev_actual.as_ref() {
                return Err(RefCASError {
                    reference: r.clone(),
                    expected: prev_expected.cloned(),
                    actual: prev_actual,
                }
                .into());
            }
        }

        self.ref_repo.set(r.as_str(), hash).await?;

        Ok(())
    }

    async fn append<'a>(
        &'a self,
        block: MetadataBlock,
        opts: AppendOpts<'a>,
    ) -> Result<Multihash, AppendError> {
        tracing::trace!(?block, "Trying to append block");

        if opts.validation == AppendValidation::Full {
            let mut validators = [
                &mut ValidateAddDataVisitor::new(&block)
                    as &mut dyn MetadataChainVisitor<Error = _>,
                &mut ValidateExecuteTransformVisitor::new(&block),
                &mut ValidateUnimplementedEventsVisitor::new(&block),
                &mut ValidateSeedBlockOrderVisitor::new(&block)?,
                &mut ValidateSequenceNumbersIntegrityVisitor::new(&block)?,
                &mut ValidateSystemTimeIsMonotonicVisitor::new(&block),
                &mut ValidateWatermarkIsMonotonicVisitor::new(&block),
                &mut ValidateEventIsNotEmptyVisitor::new(&block)?,
                &mut ValidateOffsetsAreSequentialVisitor::new(&block)?,
                &mut ValidateAddPushSourceVisitor::new(&block)?,
                &mut ValidateSetPollingSourceVisitor::new(&block)?,
                &mut ValidateSetTransformVisitor::new(&block)?,
            ];

            match self
                .accept_by_interval(&mut validators, block.prev_block_hash.as_ref(), None)
                .await
            {
                Ok(()) => Ok(()),
                Err(AcceptVisitorError::Visitor(err)) => Err(AppendError::InvalidBlock(err)),
                // Detect non-existing prev block situation
                Err(AcceptVisitorError::Traversal(IterBlocksError::BlockNotFound(err)))
                    if Some(&err.hash) == block.prev_block_hash.as_ref() =>
                {
                    Err(AppendValidationError::PrevBlockNotFound(err).into())
                }
                Err(AcceptVisitorError::Traversal(err)) => {
                    Err(AppendError::Internal(err.int_err()))
                }
            }?;
        }

        if opts.update_ref.is_some()
            && (opts.check_ref_is.is_some() || opts.check_ref_is_prev_block)
        {
            let r = opts.update_ref.unwrap();
            let expected = opts.check_ref_is.unwrap_or(block.prev_block_hash.as_ref());

            let actual = match self.ref_repo.get(r.as_str()).await {
                Ok(h) => Ok(Some(h)),
                Err(GetRefError::NotFound(_)) => Ok(None),
                Err(GetRefError::Access(e)) => Err(AppendError::Access(e)),
                Err(GetRefError::Internal(e)) => Err(AppendError::Internal(e)),
            }?;

            if expected != actual.as_ref() {
                return Err(AppendError::RefCASFailed(RefCASError {
                    reference: r.clone(),
                    expected: expected.cloned(),
                    actual,
                }));
            }
        }

        let res = self
            .meta_block_repo
            .insert_block(
                &block,
                InsertOpts {
                    precomputed_hash: opts.precomputed_hash,
                    expected_hash: opts.expected_hash,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| match e {
                InsertBlockError::HashMismatch(e) => AppendValidationError::HashMismatch(e).into(),
                InsertBlockError::Access(e) => AppendError::Access(e),
                InsertBlockError::Internal(e) => AppendError::Internal(e),
            })?;

        tracing::debug!(?block, "Successfully appended block");

        if let Some(r) = opts.update_ref {
            tracing::debug!(?r, new_hash = %res.hash, "Updating reference");
            self.ref_repo.set(r.as_str(), &res.hash).await?;
        }

        Ok(res.hash)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
