// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use std::assert_matches::assert_matches;

use kamu_cli::*;

#[test_log::test(tokio::test)]
async fn test_system_info() {
    assert_matches!(
        SystemInfo::collect(),
        SystemInfo {
            build: BuildInfo {
                app_version: kamu_cli::VERSION,
                // We trust vergen to do its job and simply ensure that we detect any incompatible
                // env var name changes
                build_timestamp: Some(_),
                git_describe: Some(_),
                git_sha: Some(_),
                git_commit_date: Some(_),
                git_branch: Some(_),
                rustc_semver: Some(_),
                rustc_channel: Some(_),
                rustc_host_triple: Some(_),
                rustc_commit_sha: Some(_),
                cargo_target_triple: Some(_),
                cargo_features: Some(_),
                cargo_opt_level: Some(_),
            }
        }
    )
}