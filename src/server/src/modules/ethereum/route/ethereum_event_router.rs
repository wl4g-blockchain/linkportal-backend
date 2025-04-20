// SPDX-License-Identifier: GNU GENERAL PUBLIC LICENSE Version 3
//
// Copyleft (c) 2024 James Wong. This file is part of James Wong.
// is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the
// Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// James Wong is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with James Wong.  If not, see <https://www.gnu.org/licenses/>.
//
// IMPORTANT: Any software that fully or partially contains or uses materials
// covered by this license must also be released under the GNU GPL license.
// This includes modifications and derived works.

use crate::modules::ethereum::handler::ethereum_event_handler::EthTransactionEventHandler;
use crate::util::web::ValidatedJson;
use crate::{
    context::state::LinkPortalState, modules::ethereum::handler::ethereum_event_handler::IEthTransactionEventHandler,
};
use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use common_telemetry::info;
use linkportal_types::modules::ethereum::ethereum_event::{
    DeleteEthTransactionEventRequest, QueryEthTransactionEventRequest, SaveEthTransactionEventRequest,
};
use linkportal_types::{
    modules::ethereum::ethereum_event::{
        DeleteEthTransactionEventResponse, QueryEthTransactionEventResponse, SaveEthTransactionEventResponse,
    },
    PageRequest,
};

pub fn init() -> Router<LinkPortalState> {
    Router::new()
        .route("/ethereum/event/query", get(handle_query_ethereum_events))
        .route("/ethereum/event/save", post(handle_save_ethereum_event))
        .route("/ethereum/event/delete", post(handle_delete_ethereum_event))
}

#[utoipa::path(
    get,
    path = "/ethereum/event/query",
    params(QueryEthTransactionEventRequest, PageRequest),
    responses((status = 200, description = "Getting for all ethereum_events.", body = QueryEthTransactionEventResponse)),
    tag = "EthEvent"
)]
async fn handle_query_ethereum_events(
    State(state): State<LinkPortalState>,
    Query(param): Query<QueryEthTransactionEventRequest>,
    Query(page): Query<PageRequest>,
) -> impl IntoResponse {
    info!("QueryEthTransactionEventRequest: {:?}, PageRequest: {:?}", param, page);
    match get_ethereum_event_handler(&state).find(param, page).await {
        Ok((page, data)) => Ok(Json(QueryEthTransactionEventResponse::new(page, data))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[utoipa::path(
    post,
    path = "/ethereum/event/save",
    request_body = SaveEthTransactionEventRequest,
    responses((status = 200, description = "Save for ethereum_event.", body = SaveEthTransactionEventResponse)),
    tag = "EthEvent"
)]
async fn handle_save_ethereum_event(
    State(state): State<LinkPortalState>,
    ValidatedJson(param): ValidatedJson<SaveEthTransactionEventRequest>,
) -> impl IntoResponse {
    info!("save ethereum_event param: {:?}", param);
    match get_ethereum_event_handler(&state).save(param).await {
        Ok(result) => Ok(Json(SaveEthTransactionEventResponse::new(result))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[utoipa::path(
    post,
    path = "/ethereum/event/delete",
    request_body = DeleteEthTransactionEventRequest,
    responses((status = 200, description = "Delete for ethereum_event.", body = DeleteEthTransactionEventResponse)),
    tag = "EthEvent"
)]
async fn handle_delete_ethereum_event(
    State(state): State<LinkPortalState>,
    Json(param): Json<DeleteEthTransactionEventRequest>,
) -> impl IntoResponse {
    info!("delete ethereum_event param: {:?}", param);
    match get_ethereum_event_handler(&state).delete(param).await {
        Ok(result) => Ok(Json(DeleteEthTransactionEventResponse::new(result))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn get_ethereum_event_handler(state: &LinkPortalState) -> Box<dyn IEthTransactionEventHandler + '_> {
    Box::new(EthTransactionEventHandler::new(state))
}
