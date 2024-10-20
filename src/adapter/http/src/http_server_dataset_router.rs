// Copyright Kamu Data, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use axum::{Extension, Json};
use database_common_macros::transactional_handler;
use dill::Catalog;
use http_common::{ApiError, IntoApiError, ResultIntoApiError};
use opendatafabric as odf;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::axum_utils::ensure_authenticated_account;
use crate::simple_protocol::*;
use crate::{DatasetAuthorizationLayer, DatasetResolverLayer};

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Extractor of dataset identity for single-tenant smart transfer protocol
#[derive(serde::Deserialize)]
struct DatasetByName {
    dataset_name: odf::DatasetName,
}

/// Extractor of account + dataset identity for multi-tenant smart transfer
/// protocol
#[derive(serde::Deserialize)]
struct DatasetByAccountAndName {
    account_name: odf::AccountName,
    dataset_name: odf::DatasetName,
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

pub fn smart_transfer_protocol_router() -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(dataset_refs_handler))
        .routes(routes!(dataset_blocks_handler))
        .routes(routes!(dataset_data_get_handler, dataset_data_put_handler))
        .routes(routes!(
            dataset_checkpoints_get_handler,
            dataset_checkpoints_put_handler
        ))
        .routes(routes!(dataset_pull_ws_upgrade_handler))
        .routes(routes!(dataset_push_ws_upgrade_handler))
        .layer(DatasetAuthorizationLayer::new(
            get_dataset_action_for_request,
        ))
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

pub fn add_dataset_resolver_layer(
    dataset_router: OpenApiRouter,
    multi_tenant: bool,
) -> OpenApiRouter {
    use axum::extract::Path;

    if multi_tenant {
        dataset_router.layer(DatasetResolverLayer::new(
            |Path(p): Path<DatasetByAccountAndName>| {
                odf::DatasetAlias::new(Some(p.account_name), p.dataset_name).into_local_ref()
            },
            is_dataset_optional_for_request,
        ))
    } else {
        dataset_router.layer(DatasetResolverLayer::new(
            |Path(p): Path<DatasetByName>| p.dataset_name.as_local_ref(),
            is_dataset_optional_for_request,
        ))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequestBody {
    pub login_method: String,
    pub login_credentials_json: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponseBody {
    pub access_token: String,
}

/// Authenticate with the node
#[utoipa::path(
    post,
    path = "/platform/login",
    request_body = LoginRequestBody,
    responses((status = OK, body = Object)),
    tag = "kamu",
    security(())
)]
#[transactional_handler]
pub async fn platform_login_handler(
    Extension(catalog): Extension<Catalog>,
    Json(payload): Json<LoginRequestBody>,
) -> Result<Json<Value>, ApiError> {
    let authentication_service = catalog
        .get_one::<dyn kamu_accounts::AuthenticationService>()
        .unwrap();

    let login_result = authentication_service
        .login(
            payload.login_method.as_str(),
            payload.login_credentials_json,
        )
        .await;

    match login_result {
        Ok(login_response) => {
            let response_body = LoginResponseBody {
                access_token: login_response.access_token,
            };
            Ok(Json(json!(response_body)))
        }
        Err(e) => Err(match e {
            kamu_accounts::LoginError::UnsupportedMethod(e) => ApiError::bad_request(e),
            kamu_accounts::LoginError::InvalidCredentials(e) => ApiError::new_unauthorized_from(e),
            kamu_accounts::LoginError::RejectedCredentials(e) => ApiError::new_unauthorized_from(e),
            kamu_accounts::LoginError::DuplicateCredentials => ApiError::bad_request(e),
            kamu_accounts::LoginError::Internal(e) => e.api_err(),
        }),
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Validate auth token
#[utoipa::path(
    get,
    path = "/platform/token/validate",
    responses((status = OK, body = ())),
    tag = "kamu",
    security(
        (),
        ("api_key" = []),
    )
)]
#[allow(clippy::unused_async)]
pub async fn platform_token_validate_handler(catalog: Extension<Catalog>) -> Result<(), ApiError> {
    ensure_authenticated_account(&catalog).api_err()?;
    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

fn is_dataset_optional_for_request(request: &http::Request<axum::body::Body>) -> bool {
    request.uri().path() == "/push"
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

fn get_dataset_action_for_request(
    request: &http::Request<axum::body::Body>,
) -> kamu_core::auth::DatasetAction {
    if !request.method().is_safe() || request.uri().path() == "/push" {
        kamu_core::auth::DatasetAction::Write
    } else {
        kamu_core::auth::DatasetAction::Read
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
