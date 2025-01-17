// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use kamu_cli_e2e_common::prelude::*;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

kamu_cli_run_api_server_e2e_test!(
    storage = mysql,
    fixture = kamu_cli_e2e_repo_tests::private_datasets::test_only_dataset_owner_can_change_dataset_visibility,
    // We need synthetic time for the tests, but the third-party JWT code
    // uses the current time. Assuming that the token lifetime is 24 hours, we will
    // use the projected date (the current day) as a workaround.
    options = Options::default()
        .with_multi_tenant()
        .with_today_as_frozen_system_time()
        .with_kamu_config(indoc::indoc!(
            r#"
            kind: CLIConfig
            version: 1
            content:
              users:
                predefined:
                  - accountName: kamu
            "#
        )),
);

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

kamu_cli_run_api_server_e2e_test!(
    storage = mysql,
    fixture =
        kamu_cli_e2e_repo_tests::private_datasets::test_admin_can_change_visibility_of_any_dataset,
    // We need synthetic time for the tests, but the third-party JWT code
    // uses the current time. Assuming that the token lifetime is 24 hours, we will
    // use the projected date (the current day) as a workaround.
    options = Options::default()
        .with_multi_tenant()
        .with_today_as_frozen_system_time()
        .with_kamu_config(indoc::indoc!(
            r#"
            kind: CLIConfig
            version: 1
            content:
              users:
                predefined:
                  - accountName: kamu
                    isAdmin: true
            "#
        )),
);

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////