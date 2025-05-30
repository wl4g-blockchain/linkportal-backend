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

use crate::context::state::LinkPortalState;
use anyhow::Error;
use async_trait::async_trait;
use common_audit_log::audit_log;
use linkportal_types::modules::ethereum::ethereum_event::{
    DeleteEthTransactionEventRequest, EthTransactionEvent, QueryEthTransactionEventRequest,
    SaveEthTransactionEventRequest,
};
use linkportal_types::{PageRequest, PageResponse};

#[async_trait]
pub trait IEthTransactionEventHandler: Send {
    async fn find(
        &self,
        param: QueryEthTransactionEventRequest,
        page: PageRequest,
    ) -> Result<(PageResponse, Vec<EthTransactionEvent>), Error>;

    async fn save(&self, param: SaveEthTransactionEventRequest) -> Result<i64, Error>;

    async fn delete(&self, param: DeleteEthTransactionEventRequest) -> Result<u64, Error>;
}

pub struct EthTransactionEventHandler<'a> {
    state: &'a LinkPortalState,
}

impl<'a> EthTransactionEventHandler<'a> {
    pub fn new(state: &'a LinkPortalState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl<'a> IEthTransactionEventHandler for EthTransactionEventHandler<'a> {
    #[audit_log("[ETH_EVENT][FIND] name: {param.transaction_hash.clone().unwrap_or_default()}")]
    async fn find(
        &self,
        param: QueryEthTransactionEventRequest,
        page: PageRequest,
    ) -> Result<(PageResponse, Vec<EthTransactionEvent>), Error> {
        let repo = self.state.eth_event_repo.read().await;
        repo.get(&self.state.config).select(param.to_event()?, page).await
    }

    #[audit_log("[ETH_EVENT][ADD] name: {param.transaction_hash.clone().unwrap_or_default()}")]
    async fn save(&self, param: SaveEthTransactionEventRequest) -> Result<i64, Error> {
        let repo = self.state.eth_event_repo.read().await;
        if param.id.is_some() {
            repo.get(&self.state.config).update(param.to_event()?).await
        } else {
            repo.get(&self.state.config).insert(param.to_event()?).await
        }
    }

    #[audit_log("[ETH_EVENT][DELETE] id: {param.id}")]
    async fn delete(&self, param: DeleteEthTransactionEventRequest) -> Result<u64, Error> {
        let repo = self.state.eth_event_repo.read().await;
        repo.get(&self.state.config).delete_by_id(param.id).await
    }
}
