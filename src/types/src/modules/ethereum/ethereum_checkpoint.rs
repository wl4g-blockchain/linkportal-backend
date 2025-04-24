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
use anyhow::Ok;
use common_makestruct::MakeStructWith;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, sqlite::SqliteRow, FromRow, Row};
use validator::Validate;

// The tx record for eth chain event.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct EthEventCheckpoint {
    #[serde(flatten)]
    pub base: BaseBean,
    pub last_processed_block: u64,
}

impl Default for EthEventCheckpoint {
    fn default() -> Self {
        EthEventCheckpoint {
            base: BaseBean::new_empty(),
            last_processed_block: 0,
        }
    }
}

/// SqliteRow impl for EthEventCheckpoint.
impl<'r> FromRow<'r, SqliteRow> for EthEventCheckpoint {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        std::result::Result::Ok(EthEventCheckpoint {
            base: BaseBean::from_row(row).unwrap(),
            last_processed_block: row.try_get("last_processed_block")?,
        })
    }
}

/// Postgres Row impl for EthEventCheckpoint.
impl<'r> FromRow<'r, PgRow> for EthEventCheckpoint {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        std::result::Result::Ok(EthEventCheckpoint {
            base: BaseBean::from_row(row)?,
            last_processed_block: row.try_get::<i64, _>("last_processed_block")? as u64, // Explicitly specify type and cast
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
pub struct QueryEthEventCheckpointRequest {
    #[validate(range(min = 1))]
    pub last_processed_block: Option<u64>,
}

impl QueryEthEventCheckpointRequest {
    pub fn to_checkpoint(&self) -> anyhow::Result<EthEventCheckpoint, anyhow::Error> {
        Ok(EthEventCheckpoint {
            base: BaseBean::new_with_by(None, None, None),
            last_processed_block: self
                .last_processed_block
                .ok_or_else(|| anyhow::anyhow!("last_processed_block is required"))?,
        })
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct QueryEthEventCheckpointResponse {
    pub page: Option<PageResponse>,
    pub data: Option<Vec<EthEventCheckpoint>>,
}

impl QueryEthEventCheckpointResponse {
    pub fn new(page: PageResponse, data: Vec<EthEventCheckpoint>) -> Self {
        QueryEthEventCheckpointResponse {
            page: Some(page),
            data: Some(data),
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Validate, utoipa::ToSchema, MakeStructWith)]
#[excludes(id)]
pub struct SaveEthEventCheckpointRequest {
    pub id: Option<i64>,
    #[validate(range(min = 1))]
    pub last_processed_block: Option<u64>,
}

impl SaveEthEventCheckpointRequest {
    pub fn to_checkpoint(&self) -> anyhow::Result<EthEventCheckpoint, anyhow::Error> {
        Ok(EthEventCheckpoint {
            base: BaseBean::new_with_id(self.id),
            last_processed_block: self
                .last_processed_block
                .ok_or_else(|| anyhow::anyhow!("last_processed_block is required"))?,
        })
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct SaveEthEventCheckpointResponse {
    pub id: i64,
}

impl SaveEthEventCheckpointResponse {
    pub fn new(id: i64) -> Self {
        SaveEthEventCheckpointResponse { id }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Validate, utoipa::ToSchema)]
pub struct DeleteEthEventCheckpointRequest {
    pub id: i64,
}

#[derive(Serialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
pub struct DeleteEthEventCheckpointResponse {
    pub count: u64,
}

impl DeleteEthEventCheckpointResponse {
    pub fn new(count: u64) -> Self {
        DeleteEthEventCheckpointResponse { count }
    }
}
