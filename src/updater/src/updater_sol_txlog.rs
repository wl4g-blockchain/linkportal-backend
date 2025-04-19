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

use crate::updater_base::IChainTxLogUpdater;
use async_trait::async_trait;
use common_telemetry::{info, warn};
use linkportal_server::config::config::UpdaterProperties;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};

#[derive(Clone)]
pub struct SolanaTxLogUpdater {
    config: Arc<UpdaterProperties>,
    scheduler: Arc<JobScheduler>,
}

impl SolanaTxLogUpdater {
    pub const KIND: &'static str = "SOLANA_TX_LOG";

    pub async fn new(config: &UpdaterProperties) -> Arc<Self> {
        // Create the this Solana compatible updater handler instance.
        Arc::new(Self {
            config: Arc::new(config.to_owned()),
            scheduler: Arc::new(JobScheduler::new_with_channel_size(config.channel_size).await.unwrap()),
        })
    }

    pub(super) async fn update(&self) {
        info!("Updating Solana compatible chain TxLog ...");
        // TODO: Implementation of the Solana compatible transaction log updater.
        todo!()
    }
}

#[async_trait]
impl IChainTxLogUpdater for SolanaTxLogUpdater {
    // start async thread job to re-scaning near real-time recorded access events.
    async fn init(&self) {
        let this = self.clone();

        // Pre-check the cron expression is valid.
        let cron = match Job::new_async(self.config.cron.as_str(), |_uuid, _lock| Box::pin(async {})) {
            Ok(_) => self.config.cron.as_str(),
            Err(e) => {
                warn!(
                    "Invalid cron expression '{}': {}. Using default '0/30 * * * * *'",
                    self.config.cron, e
                );
                "0/30 * * * * *" // every half minute
            }
        };

        info!("Starting Solana updater with cron '{}'", cron);
        let job = Job::new_async(cron, move |_uuid, _lock| {
            let that = this.clone();
            Box::pin(async move {
                that.update().await;
            })
        })
        .unwrap();

        self.scheduler.add(job).await.unwrap();
        self.scheduler.start().await.unwrap();

        info!("Started Simple LLM Analytics handler.");
        // tokio::signal::ctrl_c().await.unwrap(); // Notice: It's will keep the program running.
    }
}
