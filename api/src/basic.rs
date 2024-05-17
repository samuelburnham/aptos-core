// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::response::{BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResultWith404};
use crate::{
    accept_type::AcceptType,
    context::{api_spawn_blocking, Context},
    generate_error_response, generate_success_response,
    response::{InternalError, ServiceUnavailableError},
    ApiTags,
};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{AptosErrorCode, U64};
use aptos_crypto::HashValue;
use aptos_types::block_info::BlockHeight;
use aptos_types::transaction::Version;
use poem_openapi::{param::Query, payload::Html, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::{
    ops::Sub,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const OPEN_API_HTML: &str = include_str!("../doc/spec.html");

// Generate error and response types
generate_success_response!(HealthCheckResponse, (200, Ok));
generate_error_response!(HealthCheckError, (503, ServiceUnavailable), (500, Internal));
pub type HealthCheckResult<T> = poem::Result<HealthCheckResponse<T>, HealthCheckError>;

/// Basic API does healthchecking and shows the OpenAPI spec
pub struct BasicApi {
    pub context: Arc<Context>,
}

/// Representation of a successful healthcheck
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, Object)]
pub struct HealthCheckSuccess {
    message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, Object)]
pub struct TestPayload {
    li_version: U64,
    first_viable_version: U64,
    latest_state_checkpoint: U64,
    snapshot_before: U64,
}

impl HealthCheckSuccess {
    pub fn new() -> Self {
        Self {
            message: "aptos-node:ok".to_string(),
        }
    }
}

#[OpenApi]
impl BasicApi {
    /// Show OpenAPI explorer
    ///
    /// Provides a UI that you can use to explore the API. You can also
    /// retrieve the API directly at `/spec.yaml` and `/spec.json`.
    #[oai(
        path = "/spec",
        method = "get",
        operation_id = "spec",
        tag = "ApiTags::General"
    )]
    async fn spec(&self) -> Html<String> {
        Html(OPEN_API_HTML.to_string())
    }

    /// Check basic node health
    ///
    /// By default this endpoint just checks that it can get the latest ledger
    /// info and then returns 200.
    ///
    /// If the duration_secs param is provided, this endpoint will return a
    /// 200 if the following condition is true:
    ///
    /// `server_latest_ledger_info_timestamp >= server_current_time_timestamp - duration_secs`
    #[oai(
        path = "/-/healthy",
        method = "get",
        operation_id = "healthy",
        tag = "ApiTags::General"
    )]
    async fn healthy(
        &self,
        accept_type: AcceptType,
        /// Threshold in seconds that the server can be behind to be considered healthy
        ///
        /// If not provided, the healthcheck will always succeed
        duration_secs: Query<Option<u32>>,
    ) -> HealthCheckResult<HealthCheckSuccess> {
        let context = self.context.clone();
        let ledger_info = api_spawn_blocking(move || context.get_latest_ledger_info()).await?;

        // If we have a duration, check that it's close to the current time, otherwise it's ok
        if let Some(duration) = duration_secs.0 {
            let timestamp = ledger_info.timestamp();

            let timestamp = Duration::from_micros(timestamp);
            let expectation = SystemTime::now()
                .sub(Duration::from_secs(duration as u64))
                .duration_since(UNIX_EPOCH)
                .context("Failed to determine absolute unix time based on given duration")
                .map_err(|err| {
                    HealthCheckError::internal_with_code(
                        err,
                        AptosErrorCode::InternalError,
                        &ledger_info,
                    )
                })?;

            if timestamp < expectation {
                return Err(HealthCheckError::service_unavailable_with_code(
                    "The latest ledger info timestamp is less than the expected timestamp",
                    AptosErrorCode::HealthCheckFailed,
                    &ledger_info,
                ));
            }
        }
        HealthCheckResponse::try_from_rust_value((
            HealthCheckSuccess::new(),
            &ledger_info,
            HealthCheckResponseStatus::Ok,
            &accept_type,
        ))
    }

    #[oai(
        path = "/-/test",
        method = "get",
        operation_id = "test",
        tag = "ApiTags::General"
    )]
    async fn test(&self, accept_type: AcceptType) -> BasicResultWith404<TestPayload> {
        let (ledger_info, ledger_version, _) = self.context.state_view(None)?;

        let latest_li_w_sig = self
            .context
            .get_latest_ledger_info_with_signatures()
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        assert_eq!(latest_li_w_sig.ledger_info().version(), ledger_version, "Retrieved a latest signed ledger info with a different version than latest ledger info.");

        let (first_viable_version, _): (Version, BlockHeight) =
            self.context.db.get_first_viable_block().map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    format!("first viable block: {}", err),
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        let latest_state_checkpoint: Option<Version> = self
            .context
            .db
            .get_latest_state_checkpoint_version()
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    format!("latest state checkpoint: {}", err),
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        let snapshot_before: Option<(Version, HashValue)> = self
            .context
            .db
            .get_state_snapshot_before(ledger_version + 1)
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    format!("state snapshot before: {}", err),
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        match accept_type {
            AcceptType::Json => BasicResponse::try_from_json((
                TestPayload {
                    li_version: U64::from(ledger_version),
                    first_viable_version: U64::from(first_viable_version),
                    latest_state_checkpoint: latest_state_checkpoint
                        .or_else(|| Some(0))
                        .map(U64::from)
                        .unwrap(),
                    snapshot_before: snapshot_before
                        .or_else(|| Some((0, HashValue::default())))
                        .map(|(version, _)| U64::from(version))
                        .unwrap(),
                },
                &ledger_info,
                BasicResponseStatus::Ok,
            )),
            AcceptType::Bcs => BasicResponse::try_from_encoded((
                bcs::to_bytes(&TestPayload {
                    li_version: U64::from(ledger_version),
                    first_viable_version: U64::from(first_viable_version),
                    latest_state_checkpoint: latest_state_checkpoint
                        .or_else(|| Some(0))
                        .map(U64::from)
                        .unwrap(),
                    snapshot_before: snapshot_before
                        .or_else(|| Some((0, HashValue::default())))
                        .map(|(version, _)| U64::from(version))
                        .unwrap(),
                })
                .unwrap(),
                &ledger_info,
                BasicResponseStatus::Ok,
            )),
        }
    }
}
