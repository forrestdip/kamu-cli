// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use kamu::testing::MetadataFactory;
use kamu_cli_puppet::extensions::KamuCliPuppetExt;
use kamu_cli_puppet::KamuCliPuppet;
use opendatafabric as odf;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

pub async fn test_add_recursive(kamu: KamuCliPuppet) {
    // Plain manifest
    let snapshot = MetadataFactory::dataset_snapshot().name("plain").build();
    let manifest = odf::serde::yaml::YamlDatasetSnapshotSerializer
        .write_manifest_str(&snapshot)
        .unwrap();
    std::fs::write(kamu.workspace_path().join("plain.yaml"), manifest).unwrap();

    // Manifest with lots of comments
    let snapshot = MetadataFactory::dataset_snapshot()
        .name("commented")
        .build();
    let manifest = odf::serde::yaml::YamlDatasetSnapshotSerializer
        .write_manifest_str(&snapshot)
        .unwrap();
    std::fs::write(
        kamu.workspace_path().join("commented.yaml"),
        format!(
            indoc::indoc! {
                "

                # Some

                # Weird
                #
                # Comment
                {}
                "
            },
            &manifest
        ),
    )
    .unwrap();

    // Non-manifest YAML file
    std::fs::write(
        kamu.workspace_path().join("non-manifest.yaml"),
        indoc::indoc! {
            "
            foo:
              - bar
            "
        },
    )
    .unwrap();

    kamu.execute([
        "-v",
        "add",
        "--recursive",
        kamu.workspace_path().as_os_str().to_str().unwrap(),
    ])
    .await
    .success();

    let dataset_names = kamu
        .list_datasets()
        .await
        .into_iter()
        .map(|dataset| dataset.name)
        .collect::<Vec<_>>();

    assert_eq!(dataset_names, ["commented", "plain"]);
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
