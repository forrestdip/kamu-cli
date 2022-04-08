// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use crate::scalars::*;

use async_graphql::*;
use chrono::{DateTime, Utc};

/////////////////////////////////////////////////////////////////////////////////////////
// MetadataBlockHashed
/////////////////////////////////////////////////////////////////////////////////////////

#[derive(SimpleObject, Clone, PartialEq, Eq)]
pub struct MetadataBlockHashed {
    pub block_hash: Multihash,
    pub system_time: DateTime<Utc>,
    pub prev_block_hash: Option<Multihash>,
    pub event: MetadataEvent,
}

impl MetadataBlockHashed {
    pub fn new<H: Into<Multihash>, B: Into<MetadataBlock>>(block_hash: H, block: B) -> Self {
        let b = block.into();
        Self {
            block_hash: block_hash.into(),
            prev_block_hash: b.prev_block_hash,
            system_time: b.system_time,
            event: b.event,
        }
    }
}