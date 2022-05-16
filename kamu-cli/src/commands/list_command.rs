// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use super::{CLIError, Command};
use crate::{output::*, records_writers::TableWriter};
use kamu::domain::*;
use opendatafabric::*;

use chrono::{DateTime, Utc};
use chrono_humanize::HumanTime;
use std::sync::Arc;

pub struct ListCommand {
    dataset_reg: Arc<dyn DatasetRegistry>,
    remote_alias_reg: Arc<dyn RemoteAliasesRegistry>,
    output_config: Arc<OutputConfig>,
    detail_level: u8,
}

impl ListCommand {
    pub fn new(
        dataset_reg: Arc<dyn DatasetRegistry>,
        remote_alias_reg: Arc<dyn RemoteAliasesRegistry>,
        output_config: Arc<OutputConfig>,
        detail_level: u8,
    ) -> Self {
        Self {
            dataset_reg,
            remote_alias_reg,
            output_config,
            detail_level,
        }
    }

    // TODO: support multiple format specifiers
    async fn print_machine_readable(&self) -> Result<(), CLIError> {
        use std::io::Write;

        let mut datasets: Vec<_> = self.dataset_reg.get_all_datasets().collect();
        datasets.sort_by(|a, b| a.name.cmp(&b.name));

        let mut out = std::io::stdout();
        write!(out, "ID,Name,Kind,Head,Pulled,Records,Size\n")?;

        for hdl in &datasets {
            let head = self
                .dataset_reg
                .get_metadata_chain(&hdl.as_local_ref())?
                .read_ref(&BlockRef::Head)
                .unwrap();

            let summary = self.dataset_reg.get_summary(&hdl.as_local_ref())?;

            write!(
                out,
                "{},{},{},{},{},{},{}\n",
                hdl.id,
                hdl.name,
                self.get_kind(hdl, &summary).await?,
                head,
                match summary.last_pulled {
                    None => "".to_owned(),
                    Some(t) => t.to_rfc3339(),
                },
                summary.num_records,
                summary.data_size
            )?;
        }
        Ok(())
    }

    async fn print_pretty(&self) -> Result<(), CLIError> {
        use prettytable::*;

        let mut datasets: Vec<_> = self.dataset_reg.get_all_datasets().collect();
        datasets.sort_by(|a, b| a.name.cmp(&b.name));

        let mut table = Table::new();
        table.set_format(TableWriter::get_table_format());

        if self.detail_level == 0 {
            table.set_titles(row![bc->"Name", bc->"Kind", bc->"Pulled", bc->"Records", bc->"Size"]);
        } else {
            table.set_titles(row![bc->"ID", bc->"Name", bc->"Kind", bc->"Head", bc->"Pulled", bc->"Records", bc->"Size"]);
        }

        for hdl in datasets.iter() {
            let head = self
                .dataset_reg
                .get_metadata_chain(&hdl.as_local_ref())?
                .read_ref(&BlockRef::Head)
                .unwrap();

            let summary = self.dataset_reg.get_summary(&hdl.as_local_ref())?;

            if self.detail_level == 0 {
                table.add_row(Row::new(vec![
                    Cell::new(&hdl.name),
                    Cell::new(&self.get_kind(hdl, &summary).await?).style_spec("c"),
                    Cell::new(&self.humanize_last_pulled(summary.last_pulled)).style_spec("c"),
                    Cell::new(&self.humanize_num_records(summary.num_records)).style_spec("r"),
                    Cell::new(&self.humanize_data_size(summary.data_size)).style_spec("r"),
                ]));
            } else {
                table.add_row(Row::new(vec![
                    Cell::new(&hdl.id.to_did_string()),
                    Cell::new(&hdl.name),
                    Cell::new(&self.get_kind(hdl, &summary).await?).style_spec("c"),
                    Cell::new(&head.short().to_string()),
                    Cell::new(&self.humanize_last_pulled(summary.last_pulled)).style_spec("c"),
                    Cell::new(&self.humanize_num_records(summary.num_records)).style_spec("r"),
                    Cell::new(&self.humanize_data_size(summary.data_size)).style_spec("r"),
                ]));
            }
        }

        // Header doesn't render when there are no data rows in the table
        if datasets.is_empty() {
            if self.detail_level == 0 {
                table.add_row(Row::new(vec![
                    Cell::new(""),
                    Cell::new(""),
                    Cell::new(""),
                    Cell::new(""),
                    Cell::new(""),
                ]));
            } else {
                table.add_row(Row::new(vec![
                    Cell::new(""),
                    Cell::new(""),
                    Cell::new(""),
                    Cell::new(""),
                    Cell::new(""),
                    Cell::new(""),
                    Cell::new(""),
                ]));
            }
        }

        table.printstd();
        Ok(())
    }

    fn humanize_last_pulled(&self, last_pulled: Option<DateTime<Utc>>) -> String {
        match last_pulled {
            None => "-".to_owned(),
            Some(t) => format!("{}", HumanTime::from(t - Utc::now())),
        }
    }

    fn humanize_data_size(&self, size: u64) -> String {
        if size == 0 {
            return "-".to_owned();
        }
        use humansize::{file_size_opts, FileSize};
        size.file_size(file_size_opts::BINARY).unwrap()
    }

    fn humanize_num_records(&self, num: u64) -> String {
        use num_format::{Locale, ToFormattedString};
        if num == 0 {
            return "-".to_owned();
        }
        num.to_formatted_string(&Locale::en)
    }

    async fn get_kind(
        &self,
        handle: &DatasetHandle,
        summary: &DatasetSummary,
    ) -> Result<String, CLIError> {
        let is_remote = self
            .remote_alias_reg
            .get_remote_aliases(&handle.as_local_ref())
            .await
            .map_err(CLIError::failure)?
            .get_by_kind(RemoteAliasKind::Pull)
            .next()
            .is_some();
        Ok(if is_remote {
            format!("Remote({:?})", summary.kind)
        } else {
            format!("{:?}", summary.kind)
        })
    }
}

#[async_trait::async_trait(?Send)]
impl Command for ListCommand {
    async fn run(&mut self) -> Result<(), CLIError> {
        // TODO: replace with formatters
        match self.output_config.format {
            OutputFormat::Table => self.print_pretty().await?,
            OutputFormat::Csv => self.print_machine_readable().await?,
            _ => unimplemented!("Unsupported format: {:?}", self.output_config.format),
        }

        Ok(())
    }
}
