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

use crate::config::config::PostgresAppDBProperties;
use crate::dynamic_postgres_insert;
use crate::dynamic_postgres_query;
use crate::dynamic_postgres_update;
use crate::store::postgres::PostgresRepository;
use crate::store::AsyncRepository;
use anyhow::{Error, Ok};
use async_trait::async_trait;
use common_telemetry::info;
use linkportal_types::modules::ethereum::ethereum_event::EthTransactionEvent;
use linkportal_types::PageRequest;
use linkportal_types::PageResponse;

pub struct EthereumEventPostgresRepository {
    inner: PostgresRepository<EthTransactionEvent>,
}

impl EthereumEventPostgresRepository {
    pub async fn new(config: &PostgresAppDBProperties) -> Result<Self, Error> {
        Ok(EthereumEventPostgresRepository {
            inner: PostgresRepository::new(config).await?,
        })
    }
}

#[async_trait]
impl AsyncRepository<EthTransactionEvent> for EthereumEventPostgresRepository {
    async fn select(
        &self,
        event: EthTransactionEvent,
        page: PageRequest,
    ) -> Result<(PageResponse, Vec<EthTransactionEvent>), Error> {
        let result = dynamic_postgres_query!(
            event,
            "ch_ethereum_event",
            self.inner.get_pool(),
            "create_time",
            page,
            EthTransactionEvent
        )?;
        info!("query ch_ethereum_event: {:?}", result);
        Ok((result.0, result.1))
    }

    async fn select_by_id(&self, id: i64) -> Result<EthTransactionEvent, Error> {
        let event =
            sqlx::query_as::<_, EthTransactionEvent>("SELECT * FROM ch_ethereum_event WHERE id = $1 and del_flag = 0")
                .bind(id)
                .fetch_one(self.inner.get_pool())
                .await?;

        info!("query event: {:?}", event);
        Ok(event)
    }

    async fn insert(&self, mut event: EthTransactionEvent) -> Result<i64, Error> {
        let inserted_id = dynamic_postgres_insert!(event, "ch_ethereum_event", self.inner.get_pool())?;
        info!("Inserted event.id: {:?}", inserted_id);
        Ok(inserted_id)
    }

    async fn update(&self, mut event: EthTransactionEvent) -> Result<i64, Error> {
        let updated_id = dynamic_postgres_update!(event, "ch_ethereum_event", self.inner.get_pool())?;
        info!("Updated event.id: {:?}", updated_id);
        Ok(updated_id)
    }

    async fn delete_all(&self) -> Result<u64, Error> {
        let delete_result = sqlx::query("DELETE FROM ch_ethereum_event")
            .execute(self.inner.get_pool())
            .await?;

        info!("Deleted result: {:?}", delete_result);
        Ok(delete_result.rows_affected())
    }

    async fn delete_by_id(&self, id: i64) -> Result<u64, Error> {
        let delete_result = sqlx::query("DELETE FROM ch_ethereum_event WHERE id = $1 and del_flag = 0")
            .bind(id)
            .execute(self.inner.get_pool())
            .await?;

        info!("Deleted result: {:?}", delete_result);
        Ok(delete_result.rows_affected())
    }
}
