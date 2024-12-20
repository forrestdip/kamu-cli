// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use kamu_core::MetadataChainExt;
use kamu_flow_system::{
    CompactionRule,
    CompactionRuleFull,
    CompactionRuleMetadataOnly,
    FlowConfigurationRule,
    FlowConfigurationSnapshot,
    IngestRule,
    ResetRule,
    Schedule,
    ScheduleCron,
    ScheduleCronError,
    ScheduleTimeDelta,
    TransformRule,
};
use opendatafabric::DatasetHandle;

use crate::mutations::{FlowInvalidRunConfigurations, FlowTypeIsNotSupported};
use crate::prelude::*;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(SimpleObject, Clone, PartialEq, Eq)]
pub struct FlowConfiguration {
    pub paused: bool,
    pub ingest: Option<FlowConfigurationIngest>,
    pub transform: Option<FlowConfigurationTransform>,
    pub compaction: Option<FlowConfigurationCompaction>,
    pub reset: Option<FlowConfigurationReset>,
}

impl From<kamu_flow_system::FlowConfigurationState> for FlowConfiguration {
    fn from(value: kamu_flow_system::FlowConfigurationState) -> Self {
        Self {
            paused: !value.is_active(),
            transform: if let FlowConfigurationRule::TransformRule(condition) = &value.rule {
                Some((*condition).into())
            } else {
                None
            },
            ingest: if let FlowConfigurationRule::IngestRule(ingest_rule) = &value.rule {
                Some(ingest_rule.clone().into())
            } else {
                None
            },
            reset: if let FlowConfigurationRule::ResetRule(condition) = &value.rule {
                Some(condition.clone().into())
            } else {
                None
            },
            compaction: if let FlowConfigurationRule::CompactionRule(compaction_args) = &value.rule
            {
                match compaction_args {
                    CompactionRule::Full(compaction_rule) => {
                        Some(FlowConfigurationCompaction::Full((*compaction_rule).into()))
                    }
                    CompactionRule::MetadataOnly(compaction_rule) => Some(
                        FlowConfigurationCompaction::MetadataOnly((*compaction_rule).into()),
                    ),
                }
            } else {
                None
            },
        }
    }
}

#[derive(SimpleObject, Clone, PartialEq, Eq)]
pub struct FlowConfigurationIngest {
    pub schedule: FlowConfigurationSchedule,
    pub fetch_uncacheable: bool,
}

impl From<IngestRule> for FlowConfigurationIngest {
    fn from(value: IngestRule) -> Self {
        Self {
            fetch_uncacheable: value.fetch_uncacheable,
            schedule: match value.schedule_condition {
                Schedule::TimeDelta(time_delta) => {
                    FlowConfigurationSchedule::TimeDelta(time_delta.every.into())
                }
                Schedule::Cron(cron) => FlowConfigurationSchedule::Cron(cron.clone().into()),
            },
        }
    }
}

#[derive(Union, Clone, PartialEq, Eq)]
pub enum FlowConfigurationSchedule {
    TimeDelta(TimeDelta),
    Cron(Cron5ComponentExpression),
}

#[derive(SimpleObject, Clone, PartialEq, Eq)]
pub struct FlowConfigurationTransform {
    pub min_records_to_await: u64,
    pub max_batching_interval: TimeDelta,
}

impl From<TransformRule> for FlowConfigurationTransform {
    fn from(value: TransformRule) -> Self {
        Self {
            min_records_to_await: value.min_records_to_await(),
            max_batching_interval: (*value.max_batching_interval()).into(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(SimpleObject, Clone, PartialEq, Eq)]
pub struct FlowConfigurationReset {
    pub mode: SnapshotPropagationMode,
    pub old_head_hash: Option<Multihash>,
    pub recursive: bool,
}

#[derive(Union, Clone, PartialEq, Eq)]
pub enum SnapshotPropagationMode {
    Custom(SnapshotConfigurationResetCustom),
    ToSeed(SnapshotConfigurationResetToSeedDummy),
}

#[derive(SimpleObject, Clone, PartialEq, Eq)]
pub struct SnapshotConfigurationResetCustom {
    pub new_head_hash: Multihash,
}

#[derive(SimpleObject, Clone, PartialEq, Eq)]
pub struct SnapshotConfigurationResetToSeedDummy {
    pub dummy: String,
}

impl From<ResetRule> for FlowConfigurationReset {
    fn from(value: ResetRule) -> Self {
        let propagation_mode = if let Some(new_head_hash) = &value.new_head_hash {
            SnapshotPropagationMode::Custom(SnapshotConfigurationResetCustom {
                new_head_hash: new_head_hash.clone().into(),
            })
        } else {
            SnapshotPropagationMode::ToSeed(SnapshotConfigurationResetToSeedDummy {
                dummy: String::new(),
            })
        };
        Self {
            mode: propagation_mode,
            old_head_hash: value.old_head_hash.map(Into::into),
            recursive: value.recursive,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Union, Clone, PartialEq, Eq)]
pub enum FlowConfigurationCompaction {
    Full(CompactionFull),
    MetadataOnly(CompactionMetadataOnly),
}

#[derive(SimpleObject, Clone, PartialEq, Eq)]
pub struct CompactionFull {
    pub max_slice_size: u64,
    pub max_slice_records: u64,
    pub recursive: bool,
}

impl From<CompactionRuleFull> for CompactionFull {
    fn from(value: CompactionRuleFull) -> Self {
        Self {
            max_slice_records: value.max_slice_records(),
            max_slice_size: value.max_slice_size(),
            recursive: value.recursive(),
        }
    }
}

#[derive(SimpleObject, Clone, PartialEq, Eq)]
pub struct CompactionMetadataOnly {
    pub recursive: bool,
}

impl From<CompactionRuleMetadataOnly> for CompactionMetadataOnly {
    fn from(value: CompactionRuleMetadataOnly) -> Self {
        Self {
            recursive: value.recursive,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(SimpleObject, Clone, PartialEq, Eq)]
pub struct Cron5ComponentExpression {
    pub cron_5component_expression: String,
}

impl From<ScheduleCron> for Cron5ComponentExpression {
    fn from(value: ScheduleCron) -> Self {
        Self {
            cron_5component_expression: value.source_5component_cron_expression,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(SimpleObject, Clone, PartialEq, Eq)]
pub struct TimeDelta {
    pub every: i64,
    pub unit: TimeUnit,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Minutes,
    Hours,
    Days,
    Weeks,
}

impl From<chrono::Duration> for TimeDelta {
    fn from(value: chrono::Duration) -> Self {
        assert!(
            value.num_seconds() > 0,
            "Positive interval expected, but received [{value}]"
        );

        let num_weeks = value.num_weeks();
        if (value - chrono::Duration::try_weeks(num_weeks).unwrap()).is_zero() {
            return Self {
                every: num_weeks,
                unit: TimeUnit::Weeks,
            };
        }

        let num_days = value.num_days();
        if (value - chrono::Duration::try_days(num_days).unwrap()).is_zero() {
            return Self {
                every: num_days,
                unit: TimeUnit::Days,
            };
        }

        let num_hours = value.num_hours();
        if (value - chrono::Duration::try_hours(num_hours).unwrap()).is_zero() {
            return Self {
                every: num_hours,
                unit: TimeUnit::Hours,
            };
        }

        let num_minutes = value.num_minutes();
        if (value - chrono::Duration::try_minutes(num_minutes).unwrap()).is_zero() {
            return Self {
                every: num_minutes,
                unit: TimeUnit::Minutes,
            };
        }

        panic!(
            "Expecting intervals not smaller than 1 minute that are clearly dividable by unit, \
             but received [{value}]"
        );
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(OneofObject)]
pub enum FlowRunConfiguration {
    Transform(TransformConditionInput),
    Compaction(CompactionConditionInput),
    Ingest(IngestConditionInput),
    Reset(ResetConditionInput),
}

#[derive(OneofObject, Clone)]
pub enum ScheduleInput {
    TimeDelta(TimeDeltaInput),
    /// Supported CRON syntax: min hour dayOfMonth month dayOfWeek
    Cron5ComponentExpression(String),
}

#[derive(InputObject, Clone)]
pub struct TimeDeltaInput {
    pub every: u32,
    pub unit: TimeUnit,
}

impl From<TimeDeltaInput> for chrono::Duration {
    fn from(value: TimeDeltaInput) -> Self {
        let every = i64::from(value.every);
        match value.unit {
            TimeUnit::Weeks => chrono::Duration::try_weeks(every).unwrap(),
            TimeUnit::Days => chrono::Duration::try_days(every).unwrap(),
            TimeUnit::Hours => chrono::Duration::try_hours(every).unwrap(),
            TimeUnit::Minutes => chrono::Duration::try_minutes(every).unwrap(),
        }
    }
}

impl From<&TimeDeltaInput> for chrono::Duration {
    fn from(value: &TimeDeltaInput) -> Self {
        let every = i64::from(value.every);
        match value.unit {
            TimeUnit::Weeks => chrono::Duration::try_weeks(every).unwrap(),
            TimeUnit::Days => chrono::Duration::try_days(every).unwrap(),
            TimeUnit::Hours => chrono::Duration::try_hours(every).unwrap(),
            TimeUnit::Minutes => chrono::Duration::try_minutes(every).unwrap(),
        }
    }
}

#[derive(InputObject, Clone)]
pub struct TransformConditionInput {
    pub min_records_to_await: u64,
    pub max_batching_interval: TimeDeltaInput,
}

impl From<TransformConditionInput> for FlowRunConfiguration {
    fn from(value: TransformConditionInput) -> Self {
        Self::Transform(value)
    }
}

#[derive(OneofObject, Clone)]
pub enum PropagationMode {
    Custom(FlowConfigurationResetCustom),
    ToSeed(FlowConfigurationResetToSeedDummy),
}

#[derive(InputObject, Clone)]
pub struct FlowConfigurationResetCustom {
    pub new_head_hash: Multihash,
}

#[derive(InputObject, Clone)]
pub struct FlowConfigurationResetToSeedDummy {
    dummy: String,
}

#[derive(InputObject, Clone)]
pub struct ResetConditionInput {
    pub mode: PropagationMode,
    pub old_head_hash: Option<Multihash>,
    pub recursive: bool,
}

impl ResetConditionInput {
    pub fn new_head_hash(&self) -> Option<Multihash> {
        match &self.mode {
            PropagationMode::Custom(custom_args) => Some(custom_args.new_head_hash.clone()),
            PropagationMode::ToSeed(_) => None,
        }
    }
}

impl From<ResetConditionInput> for FlowRunConfiguration {
    fn from(value: ResetConditionInput) -> Self {
        Self::Reset(value)
    }
}

#[derive(OneofObject, Clone)]
pub enum CompactionConditionInput {
    Full(CompactionConditionFull),
    MetadataOnly(CompactionConditionMetadataOnly),
}

#[derive(InputObject, Clone)]
pub struct CompactionConditionFull {
    pub max_slice_size: u64,
    pub max_slice_records: u64,
    pub recursive: bool,
}

#[derive(InputObject, Clone)]
pub struct CompactionConditionMetadataOnly {
    pub recursive: bool,
}

impl From<CompactionConditionInput> for FlowRunConfiguration {
    fn from(value: CompactionConditionInput) -> Self {
        Self::Compaction(value)
    }
}

#[derive(InputObject, Clone)]
pub struct IngestConditionInput {
    /// Flag indicates to ignore cache during ingest step for API calls
    pub fetch_uncacheable: bool,
    pub schedule: ScheduleInput,
}

impl TryFrom<IngestConditionInput> for IngestRule {
    type Error = ScheduleCronError;

    fn try_from(value: IngestConditionInput) -> std::result::Result<Self, Self::Error> {
        let schedule = match value.schedule {
            ScheduleInput::TimeDelta(td) => {
                Schedule::TimeDelta(ScheduleTimeDelta { every: td.into() })
            }
            ScheduleInput::Cron5ComponentExpression(cron_5component_expression) => {
                Schedule::try_from_5component_cron_expression(&cron_5component_expression)?
            }
        };
        Ok(Self {
            fetch_uncacheable: value.fetch_uncacheable,
            schedule_condition: schedule,
        })
    }
}

impl From<IngestConditionInput> for FlowRunConfiguration {
    fn from(value: IngestConditionInput) -> Self {
        Self::Ingest(value)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

impl FlowRunConfiguration {
    pub async fn try_into_snapshot(
        ctx: &Context<'_>,
        dataset_flow_type: &DatasetFlowType,
        dataset_handle: &DatasetHandle,
        flow_run_configuration_maybe: Option<&FlowRunConfiguration>,
    ) -> Result<Option<FlowConfigurationSnapshot>, FlowInvalidRunConfigurations> {
        match dataset_flow_type {
            DatasetFlowType::Ingest => {
                if let Some(flow_run_configuration) = flow_run_configuration_maybe {
                    if let Self::Ingest(ingest_input) = flow_run_configuration {
                        return Ok(Some(FlowConfigurationSnapshot::Ingest(IngestRule {
                            fetch_uncacheable: ingest_input.fetch_uncacheable,
                            schedule_condition: match &ingest_input.schedule {
                                ScheduleInput::TimeDelta(td) => {
                                    Schedule::TimeDelta(ScheduleTimeDelta { every: td.into() })
                                }
                                ScheduleInput::Cron5ComponentExpression(
                                    cron_5component_expression,
                                ) => Schedule::try_from_5component_cron_expression(
                                    cron_5component_expression,
                                )
                                .map_err(|_| {
                                    FlowInvalidRunConfigurations {
                                        error: "Invalid schedule flow run configuration"
                                            .to_string(),
                                    }
                                })?,
                            },
                        })));
                    }
                    return Err(FlowInvalidRunConfigurations {
                        error: "Incompatible flow run configuration and dataset flow type"
                            .to_string(),
                    });
                }
            }
            DatasetFlowType::ExecuteTransform => {
                if let Some(flow_run_configuration) = flow_run_configuration_maybe {
                    if let Self::Transform(transform_input) = flow_run_configuration {
                        return Ok(Some(FlowConfigurationSnapshot::Transform(
                            TransformRule::new_checked(
                                transform_input.min_records_to_await,
                                transform_input.max_batching_interval.clone().into(),
                            )
                            .map_err(|_| {
                                FlowInvalidRunConfigurations {
                                    error: "Invalid transform flow run configuration".to_string(),
                                }
                            })?,
                        )));
                    }
                    return Err(FlowInvalidRunConfigurations {
                        error: "Incompatible flow run configuration and dataset flow type"
                            .to_string(),
                    });
                }
            }
            DatasetFlowType::HardCompaction => {
                if let Some(flow_run_configuration) = flow_run_configuration_maybe {
                    if let Self::Compaction(compaction_input) = flow_run_configuration {
                        return Ok(Some(FlowConfigurationSnapshot::Compaction(
                            match compaction_input {
                                CompactionConditionInput::Full(compaction_input) => {
                                    CompactionRule::Full(
                                        CompactionRuleFull::new_checked(
                                            compaction_input.max_slice_size,
                                            compaction_input.max_slice_records,
                                            compaction_input.recursive,
                                        )
                                        .map_err(|_| {
                                            FlowInvalidRunConfigurations {
                                                error: "Invalid compaction flow run configuration"
                                                    .to_string(),
                                            }
                                        })?,
                                    )
                                }
                                CompactionConditionInput::MetadataOnly(compaction_input) => {
                                    CompactionRule::MetadataOnly(CompactionRuleMetadataOnly {
                                        recursive: compaction_input.recursive,
                                    })
                                }
                            },
                        )));
                    }
                    return Err(FlowInvalidRunConfigurations {
                        error: "Incompatible flow run configuration and dataset flow type"
                            .to_string(),
                    });
                }
            }
            DatasetFlowType::Reset => {
                let dataset_registry = from_catalog::<dyn kamu_core::DatasetRegistry>(ctx).unwrap();
                let resolved_dataset = dataset_registry.get_dataset_by_handle(dataset_handle);

                // Assume unwrap safe such as we have checked this existance during
                // validation step
                let current_head_hash = resolved_dataset
                    .as_metadata_chain()
                    .try_get_ref(&kamu_core::BlockRef::Head)
                    .await
                    .map_err(|_| FlowInvalidRunConfigurations {
                        error: "Cannot fetch default value".to_string(),
                    })?;
                if let Some(flow_run_configuration) = flow_run_configuration_maybe {
                    if let Self::Reset(reset_input) = flow_run_configuration {
                        let old_head_hash = if reset_input.old_head_hash.is_some() {
                            reset_input.old_head_hash.clone().map(Into::into)
                        } else {
                            current_head_hash
                        };
                        return Ok(Some(FlowConfigurationSnapshot::Reset(ResetRule {
                            new_head_hash: reset_input.new_head_hash().map(Into::into),
                            old_head_hash,
                            recursive: reset_input.recursive,
                        })));
                    }
                    return Err(FlowInvalidRunConfigurations {
                        error: "Incompatible flow run configuration and dataset flow type"
                            .to_string(),
                    });
                }
                return Ok(Some(FlowConfigurationSnapshot::Reset(ResetRule {
                    new_head_hash: None,
                    old_head_hash: current_head_hash,
                    recursive: false,
                })));
            }
        }
        Ok(None)
    }

    pub fn check_type_compatible(
        &self,
        flow_type: DatasetFlowType,
    ) -> Result<(), FlowTypeIsNotSupported> {
        match self {
            Self::Ingest(_) => {
                if flow_type == DatasetFlowType::Ingest {
                    return Ok(());
                }
            }
            Self::Transform(_) => {
                if flow_type == DatasetFlowType::ExecuteTransform {
                    return Ok(());
                }
            }
            Self::Compaction(_) => {
                if flow_type == DatasetFlowType::HardCompaction {
                    return Ok(());
                }
            }
            Self::Reset(_) => {
                if flow_type == DatasetFlowType::Reset {
                    return Ok(());
                }
            }
        }
        Err(FlowTypeIsNotSupported)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
