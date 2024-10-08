// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use clap::Parser as _;

fn main() {
    let workspace_layout = kamu_cli::WorkspaceService::find_workspace();
    let args = kamu_cli::cli::Cli::parse();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(kamu_cli::run(workspace_layout, args));

    match result {
        Ok(_) => (),
        Err(_) => {
            std::process::exit(1);
        }
    }
}
