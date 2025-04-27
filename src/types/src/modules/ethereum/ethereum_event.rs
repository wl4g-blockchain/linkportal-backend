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

use crate::{BaseBean, PageResponse};
use anyhow::{Error, Ok};
use common_makestruct::MakeStructWith;
use ethers::{abi::Abi, types::H160};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgRow, sqlite::SqliteRow, FromRow, Row};
use validator::Validate;

// The tx record for eth chain event.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct EthTransactionEvent {
    #[serde(flatten)]
    pub base: BaseBean,
    pub block_number: u64,
    pub transaction_hash: String,
    pub contract_name: String,
    pub contract_address: String,
    pub event_name: String,
    pub event_data: Value, // Event JSON data.
}

impl Default for EthTransactionEvent {
    fn default() -> Self {
        EthTransactionEvent {
            base: BaseBean::new_with_id(None),
            block_number: 0,
            transaction_hash: "".to_string(),
            contract_name: "".to_string(),
            contract_address: "".to_string(),
            event_name: "".to_string(),
            event_data: Value::Null,
        }
    }
}

/// SqliteRow impl for EthTransactionEvent.
impl<'r> FromRow<'r, SqliteRow> for EthTransactionEvent {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        let mut base = BaseBean::from_row(row).expect("Failed to deserialize BaseBean");
        base.del_flag = row.try_get::<Option<bool>, _>("removed")?.map(|v| v as i32);
        std::result::Result::Ok(EthTransactionEvent {
            base,
            block_number: row.try_get("block_number")?,
            transaction_hash: row.try_get("transaction_hash")?,
            contract_name: row.try_get("contract_name")?,
            contract_address: row.try_get("contract_address")?,
            event_name: row.try_get("event_name")?,
            event_data: row.try_get("event_data")?,
        })
    }
}

/// Postgres Row impl for EthTransactionEvent.
impl<'r> FromRow<'r, PgRow> for EthTransactionEvent {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        let mut base = BaseBean::from_row(row).expect("Failed to deserialize BaseBean");
        base.del_flag = row.try_get::<Option<bool>, _>("removed")?.map(|v| v as i32);
        std::result::Result::Ok(EthTransactionEvent {
            base,
            block_number: row.try_get::<i64, _>("block_number")? as u64, // Explicitly specify type and cast
            transaction_hash: row.try_get("transaction_hash")?,
            contract_name: row.try_get("contract_name")?,
            contract_address: row.try_get("contract_address")?,
            event_name: row.try_get("event_name")?,
            event_data: row.try_get("event_data")?,
        })
    }
}

#[derive(
    Deserialize,
    Clone,
    Debug,
    PartialEq,
    Validate,
    utoipa::ToSchema,
    utoipa::IntoParams, // PageableQueryRequest // Try using macro auto generated pageable query request.
)]
#[into_params(parameter_in = Query)]
pub struct QueryEthTransactionEventRequest {
    #[validate(range(min = 1))]
    pub block_number: Option<u64>,
    #[validate(length(min = 1, max = 256))]
    pub transaction_hash: Option<String>,
    #[validate(length(min = 1, max = 32))]
    pub contract_name: Option<String>,
    #[validate(length(min = 1, max = 32))]
    pub contract_address: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub event_name: Option<String>,
}

impl QueryEthTransactionEventRequest {
    pub fn to_event(&self) -> anyhow::Result<EthTransactionEvent, anyhow::Error> {
        Ok(EthTransactionEvent {
            base: BaseBean::new_with_by(None, None, None),
            block_number: self
                .block_number
                .ok_or_else(|| anyhow::anyhow!("block_number is required"))?,
            transaction_hash: self
                .transaction_hash
                .to_owned()
                .ok_or_else(|| anyhow::anyhow!("transaction_hash is required"))?,
            contract_name: self
                .contract_name
                .to_owned()
                .ok_or_else(|| anyhow::anyhow!("contract_name is required"))?,
            contract_address: self
                .contract_address
                .to_owned()
                .ok_or_else(|| anyhow::anyhow!("contract_address is required"))?,
            event_name: self
                .event_name
                .to_owned()
                .ok_or_else(|| anyhow::anyhow!("event_name is required"))?,
            event_data: Value::Null,
        })
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct QueryEthTransactionEventResponse {
    pub page: Option<PageResponse>,
    pub data: Option<Vec<EthTransactionEvent>>,
}

impl QueryEthTransactionEventResponse {
    pub fn new(page: PageResponse, data: Vec<EthTransactionEvent>) -> Self {
        QueryEthTransactionEventResponse {
            page: Some(page),
            data: Some(data),
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Validate, utoipa::ToSchema, MakeStructWith)]
#[excludes(id)]
pub struct SaveEthTransactionEventRequest {
    pub id: Option<i64>,
    #[validate(range(min = 1))]
    pub block_number: Option<u64>,
    #[validate(length(min = 1, max = 256))]
    pub transaction_hash: Option<String>,
    #[validate(length(min = 1, max = 32))]
    pub contract_name: Option<String>,
    #[validate(length(min = 1, max = 32))]
    pub contract_address: Option<String>,
    #[validate(length(min = 1, max = 64))]
    pub event_name: Option<String>,
    pub event_data: Option<Value>,
}

impl SaveEthTransactionEventRequest {
    pub fn to_event(&self) -> anyhow::Result<EthTransactionEvent, Error> {
        Ok(EthTransactionEvent {
            base: BaseBean::new_with_id(self.id),
            block_number: self
                .block_number
                .ok_or_else(|| anyhow::anyhow!("block_number is required"))?,
            transaction_hash: self
                .transaction_hash
                .to_owned()
                .ok_or_else(|| anyhow::anyhow!("transaction_hash is required"))?,
            contract_name: self
                .contract_name
                .to_owned()
                .ok_or_else(|| anyhow::anyhow!("contract_name is required"))?,
            contract_address: self
                .contract_address
                .to_owned()
                .ok_or_else(|| anyhow::anyhow!("contract_address is required"))?,
            event_name: self
                .event_name
                .to_owned()
                .ok_or_else(|| anyhow::anyhow!("event_name is required"))?,
            event_data: self
                .event_data
                .to_owned()
                .ok_or_else(|| anyhow::anyhow!("event_data is required"))?,
        })
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct SaveEthTransactionEventResponse {
    pub id: i64,
}

impl SaveEthTransactionEventResponse {
    pub fn new(id: i64) -> Self {
        SaveEthTransactionEventResponse { id }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Validate, utoipa::ToSchema)]
pub struct DeleteEthTransactionEventRequest {
    pub id: i64,
}

#[derive(Serialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct DeleteEthTransactionEventResponse {
    pub count: u64,
}

impl DeleteEthTransactionEventResponse {
    pub fn new(count: u64) -> Self {
        DeleteEthTransactionEventResponse { count }
    }
}

/// The Contract watch configuration
pub struct EthContractSpec {
    pub name: String,
    pub address: H160,
    pub abi: Abi,
    pub filter_events: Vec<String>, // Target watch chain event names.
}
