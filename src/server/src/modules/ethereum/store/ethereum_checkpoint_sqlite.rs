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

use crate::config::config::SqliteAppDBProperties;
use crate::dynamic_sqlite_insert;
use crate::dynamic_sqlite_query;
use crate::dynamic_sqlite_update;
use crate::store::sqlite::SQLiteRepository;
use crate::store::AsyncRepository;
use anyhow::{Error, Ok};
use async_trait::async_trait;
use common_telemetry::info;
use linkportal_types::modules::ethereum::ethereum_checkpoint::EthEventCheckpoint;
use linkportal_types::PageRequest;
use linkportal_types::PageResponse;

pub struct EthereumCheckpointSQLiteRepository {
    inner: SQLiteRepository<EthEventCheckpoint>,
}

impl EthereumCheckpointSQLiteRepository {
    pub async fn new(config: &SqliteAppDBProperties) -> Result<Self, Error> {
        Ok(EthereumCheckpointSQLiteRepository {
            inner: SQLiteRepository::new(config).await?,
        })
    }
}

#[async_trait]
impl AsyncRepository<EthEventCheckpoint> for EthereumCheckpointSQLiteRepository {
    async fn select(
        &self,
        checkpoint: EthEventCheckpoint,
        page: PageRequest,
    ) -> Result<(PageResponse, Vec<EthEventCheckpoint>), Error> {
        let result = dynamic_sqlite_query!(
            checkpoint,
            "ch_ethereum_checkpoint",
            self.inner.get_pool(),
            "update_time",
            page,
            EthEventCheckpoint
        )?;

        info!("query ch_ethereum_checkpoint: {:?}", result);
        Ok((result.0, result.1))
    }

    async fn select_by_id(&self, id: i64) -> Result<EthEventCheckpoint, Error> {
        let checkpoint = sqlx::query_as::<_, EthEventCheckpoint>(
            "SELECT * FROM ch_ethereum_checkpoint WHERE id = $1 and del_flag = 0",
        )
        .bind(id)
        .fetch_one(self.inner.get_pool())
        .await?;

        info!("query checkpoint: {:?}", checkpoint);
        Ok(checkpoint)
    }

    async fn insert(&self, mut checkpoint: EthEventCheckpoint) -> Result<i64, Error> {
        let inserted_id = dynamic_sqlite_insert!(checkpoint, "ch_ethereum_checkpoint", self.inner.get_pool())?;
        info!("Inserted checkpoint.id: {:?}", inserted_id);
        Ok(inserted_id)
    }

    async fn update(&self, mut checkpoint: EthEventCheckpoint) -> Result<i64, Error> {
        let updated_id = dynamic_sqlite_update!(checkpoint, "ch_ethereum_checkpoint", self.inner.get_pool())?;
        info!("Updated checkpoint.id: {:?}", updated_id);
        Ok(updated_id)
    }

    async fn delete_all(&self) -> Result<u64, Error> {
        let delete_result = sqlx::query("DELETE FROM ch_ethereum_checkpoint")
            .execute(self.inner.get_pool())
            .await?;

        info!("Deleted result: {:?}", delete_result);
        Ok(delete_result.rows_affected())
    }

    async fn delete_by_id(&self, id: i64) -> Result<u64, Error> {
        let delete_result = sqlx::query("DELETE FROM ch_ethereum_checkpoint WHERE id = $1 and del_flag = 0")
            .bind(id)
            .execute(self.inner.get_pool())
            .await?;

        info!("Deleted result: {:?}", delete_result);
        Ok(delete_result.rows_affected())
    }
}
