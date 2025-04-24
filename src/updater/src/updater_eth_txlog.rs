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

use super::updater_base::IChainTxLogUpdater;
use anyhow::{Context, Ok};
use async_trait::async_trait;
use common_telemetry::{debug, error, info, warn};
use ethers::{
    abi::{Abi, RawLog},
    providers::{Http, Middleware, Provider, StreamExt, Ws},
    types::{BlockNumber, Filter, FilterBlockOption, Log, ValueOrArray, U64},
};
use linkportal_server::{
    config::config::{AppConfig, EthereumChainProperties, UpdaterProperties},
    context::state::LinkPortalState,
    store::RepositoryContainer,
};
use linkportal_types::{modules::ethereum::ethereum_checkpoint::EthEventCheckpoint, BaseBean};
use linkportal_types::{
    modules::ethereum::ethereum_event::{EthContractSpec, EthTransactionEvent},
    PageRequest,
};
use serde::de;
use serde_json::{json, Value};
use std::{fs::File, io::BufReader, sync::Arc};
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing_subscriber::field::debug;

#[derive(Clone)]
pub struct EthereumTxLogUpdater {
    config: Arc<AppConfig>,
    updater_config: Arc<UpdaterProperties>,
    chain_config: Arc<EthereumChainProperties>,
    scheduler: Arc<JobScheduler>,
    contract_specs: Arc<Vec<EthContractSpec>>,
    eth_event_repo: Arc<RwLock<RepositoryContainer<EthTransactionEvent>>>,
    eth_checkpoint_repo: Arc<RwLock<RepositoryContainer<EthEventCheckpoint>>>,
}

impl EthereumTxLogUpdater {
    pub const KIND: &'static str = "ETHEREUM";

    pub async fn new(config: Arc<AppConfig>, updater_config: &UpdaterProperties) -> Arc<Self> {
        let chain_config = updater_config.chain.as_ethereum();

        let contract_specs = chain_config
            .contracts
            .iter()
            .flat_map(|cs| cs.iter())
            .map(|c| {
                let abi: Abi = serde_json::from_reader(BufReader::new(
                    File::open(&c.abi_path).expect(format!("Failed to read ABI file: {}", c.abi_path).as_str()),
                ))
                .expect("Failed to read ABI from contract json.");
                EthContractSpec {
                    address: c.address.parse().expect("Failed to parse contract address"),
                    abi: abi.to_owned(),
                    filter_events: c.event_names.to_owned(),
                }
            })
            .collect();

        // Create the this Ethereum chain TxLog updater instance.
        Arc::new(Self {
            config: config.to_owned(),
            updater_config: Arc::new(updater_config.to_owned()),
            chain_config: Arc::new(chain_config.to_owned()),
            scheduler: Arc::new(
                JobScheduler::new_with_channel_size(updater_config.channel_size)
                    .await
                    .expect("Failed to create JobScheduler"),
            ),
            contract_specs: Arc::new(contract_specs),
            eth_event_repo: Arc::new(RwLock::new(LinkPortalState::new_eth_event_repo(&config.appdb).await)),
            eth_checkpoint_repo: Arc::new(RwLock::new(
                LinkPortalState::new_eth_checkpoint_repo(&config.appdb).await,
            )),
        })
    }

    pub(super) async fn update(&self) {
        info!("Updating Ethereum chain TxLog ...");
        if let Err(e) = self.http_block_poller().await {
            error!("Failed to http poll update block TxLog: {:?}", e);
        }
    }

    async fn ws_block_listener(&self) {
        // Initialize the Ethereum RPC providers.
        let provider_ws = Arc::new(
            Provider::<Ws>::connect(self.chain_config.ws_rpc_url.as_str())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create WebSocket provider: {}", e))
                .expect("Failed to create WebSocket provider"),
        );

        // Load the last persist checkpoint.
        let last_block = self.load_checkpoint().await.expect("Failed to load checkpoint");

        let mut subscribe_stream = provider_ws
            .subscribe_logs(&self.build_logs_filter(last_block))
            .await
            .expect("Failed to subscribe to Ethereum logs with WebSocket");

        let mut block_number = 0;
        let mut events = Vec::new();
        while let Some(log) = subscribe_stream.next().await {
            info!("Subscribe TxLog: {:?}", log);
            block_number = log.block_number.unwrap().as_u64();
            // skip the block if it is less than the last checkpoint.
            if block_number <= last_block {
                continue;
            }
            match self.parse_log_as_event(log.to_owned()).await {
                anyhow::Result::Ok(op) => {
                    if let Some(event) = op {
                        debug!("Parsed event: {:?}", event);
                        events.push(event);
                    } else {
                        debug!("Ignore the parse event of log: {:?}", log);
                    }
                }
                Err(e) => {
                    warn!("Unable to parse log with ws. {}: {:?}", block_number, e);
                    continue;
                }
            }
        }
        if !events.is_empty() {
            return;
        }

        // Persist the events to DB.
        match self.save_events_batch(&events).await {
            anyhow::Result::Ok(_) => match self.save_checkpoint(block_number).await {
                anyhow::Result::Ok(_) => {
                    info!("Persisted checkpoint the block: {}", block_number);
                }
                Err(e) => {
                    error!("Error persist with block {}: {:?}", block_number, e);
                }
            },
            Err(e) => {
                error!("Error persist with block {}: {:?}", block_number, e);
            }
        };

        // Handle the chain transaction rollback.
        match self.handle_rollback(&events).await {
            anyhow::Result::Ok(_) => {
                info!("Finshed the handle rollback with events: {}", &events.iter().count());
            }
            Err(e) => {
                info!(
                    "Error process rollback with events: {}, {:?}",
                    &events.iter().count(),
                    e
                );
            }
        };
    }

    async fn http_block_poller(&self) -> anyhow::Result<()> {
        // Load the last persist checkpoint.
        let mut last_block = self.load_checkpoint().await.expect("Failed to load checkpoint");

        let provider_http = Arc::new(
            Provider::<Http>::try_from(&self.chain_config.http_rpc_url).expect("Failed to create HTTP provider"),
        );
        let current_block = provider_http.get_block_number().await?.as_u64();
        let mut events = Vec::new();

        // Handle the missing blocks.
        while last_block < current_block {
            last_block += 1;
            info!("Polling block number: {}", last_block);
            for log in provider_http.get_logs(&self.build_logs_filter(last_block)).await? {
                match self.parse_log_as_event(log.to_owned()).await {
                    anyhow::Result::Ok(op) => {
                        if let Some(event) = op {
                            debug!("Parsed event: {:?}", event);
                            events.push(event);
                        } else {
                            debug!("Ignore the parse event of log: {:?}", log);
                        }
                    }
                    Err(e) => {
                        warn!("Unable to parse log with http. {}: {:?}", last_block, e);
                        continue;
                    }
                }
            }
            // Persist the events to DB.
            match self.save_events_batch(&events).await {
                anyhow::Result::Ok(_) => match self.save_checkpoint(last_block).await {
                    anyhow::Result::Ok(_) => {
                        info!("Persisted checkpoint the block: {}", last_block);
                    }
                    Err(e) => {
                        error!("Error persist with block {}: {:?}", last_block, e);
                    }
                },
                Err(e) => {
                    error!("Error persist with block {}: {:?}", last_block, e);
                }
            };
            // Handle the chain transaction rollback.
            match self.handle_rollback(&events).await {
                anyhow::Result::Ok(_) => {
                    info!("Finshed the process rollback with events: {}", &events.iter().count());
                }
                Err(e) => {
                    info!(
                        "Error process rollback with events: {}, {:?}",
                        &events.iter().count(),
                        e
                    );
                }
            };
        }

        todo!()
    }

    fn build_logs_filter(&self, last_block: u64) -> Filter {
        let mut filter = Filter::default();
        filter.address = Some(ValueOrArray::Array(
            self.contract_specs.iter().map(|c| c.address).collect(),
        ));
        filter.block_option = FilterBlockOption::Range {
            from_block: Some(BlockNumber::Number(U64::from(last_block))),
            to_block: None,
        };
        return filter;
    }

    async fn parse_log_as_event(&self, log: Log) -> anyhow::Result<Option<EthTransactionEvent>, anyhow::Error> {
        // Match the contract address again.
        let spec = self
            .contract_specs
            .iter()
            .find(|c| c.address == log.address)
            .ok_or_else(|| anyhow::anyhow!("Contract spec not found for address: {:?}", log.address))?;

        // Match the event signature.
        let abi_event = spec
            .abi
            .events()
            .find(|e| e.signature() == log.topics[0])
            .ok_or_else(|| anyhow::anyhow!("Event not found for signature: {:?}", log.topics[0]))?;

        // This event is not what we wanted.
        if !spec.filter_events.contains(&abi_event.name) {
            return Ok(None);
        }

        // Parse raw log to abi event.
        let abi_log = abi_event
            .parse_log(RawLog {
                topics: log.topics,
                data: log.data.to_vec(),
            })
            .context(format!(
                "Could not parse log to Ethereum event with name: {}",
                abi_event.name
            ))?;

        // Transform the log params to json map.
        let mut event_map = serde_json::Map::new();
        for (event_param, log_param) in abi_event.inputs.iter().zip(abi_log.params) {
            let value = match serde_json::to_value(&log_param.value) {
                anyhow::Result::Ok(v) => v,
                Err(_) => json!(format!("{:?}", &log_param.value)),
            };
            event_map.insert(event_param.name.to_owned(), value);
        }

        Ok(Some(EthTransactionEvent {
            base: BaseBean::new_with_id(None),
            block_number: log.block_number.unwrap().as_u64(),
            transaction_hash: format!("{:?}", log.transaction_hash.unwrap()),
            contract_address: format!("{:?}", log.address),
            event_name: abi_event.name.clone(),
            event_data: Value::Object(event_map),
        }))
    }

    async fn load_checkpoint(&self) -> anyhow::Result<u64> {
        let eth_checkpoint_repo = self.eth_checkpoint_repo.read().await;
        let checkpoint = eth_checkpoint_repo
            .get(&self.config)
            .select(EthEventCheckpoint::default(), PageRequest::default())
            .await;

        // if there are no checkpoint, set the default last processed block to 0.
        match checkpoint {
            std::result::Result::Ok(checkpoint) => {
                if checkpoint.1.is_empty() {
                    Ok(0)
                } else {
                    Ok(checkpoint.1.last().map(|e| e.last_processed_block).unwrap_or(0))
                }
            }
            Err(e) => {
                warn!("Failed to get checkpoint for contract {:?}", e);
                Ok(0)
            }
        }
    }

    async fn save_checkpoint(&self, block_number: u64) -> anyhow::Result<()> {
        let mut checkpoint = EthEventCheckpoint::default();
        checkpoint.last_processed_block = block_number;
        self.eth_checkpoint_repo
            .write()
            .await
            .get(&self.config)
            .insert(checkpoint)
            .await?;
        Ok(())
    }

    async fn save_events_batch(&self, events: &Vec<EthTransactionEvent>) -> anyhow::Result<()> {
        // TODO: impl batch inserts into DB.
        for event in events.iter() {
            self.eth_event_repo
                .write()
                .await
                .get(&self.config)
                .insert(event.to_owned())
                .await?;
        }
        Ok(())
    }

    async fn handle_rollback(&self, events: &Vec<EthTransactionEvent>) -> anyhow::Result<()> {
        // 删除比新区块号更大的所有区块数据
        // let deleted = sqlx::query("DELETE FROM contract_events WHERE block_number > $1")
        //     .bind(block_number as i64)
        //     .execute(pool)
        //     .await?;
        // if deleted.rows_affected() > 0 {
        //     info!(
        //         "Reorg detected. Removed {} events from reorged blocks",
        //         deleted.rows_affected()
        //     );
        // }
        // Ok(())
        // TODO: impl batch inserts into DB.
        for event in events.iter() {
            self.eth_event_repo
                .write()
                .await
                .get(&self.config)
                .insert(event.to_owned())
                .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl IChainTxLogUpdater for EthereumTxLogUpdater {
    async fn init(&self) {
        let this = self.clone();

        // Pre-check the cron expression is valid.
        let cron = match Job::new_async(self.updater_config.cron.as_str(), |_uuid, _lock| Box::pin(async {})) {
            anyhow::Result::Ok(_) => self.updater_config.cron.as_str(),
            Err(e) => {
                warn!(
                    "Invalid cron expression '{}': {}. Using default '0/30 * * * * *'",
                    self.updater_config.cron, e
                );
                "0 5 * * * *" // every 5 minute
            }
        };

        info!("Starting Ethereum updater with cron '{}'", cron);
        let job = Job::new_async(cron, move |_uuid, _lock| {
            let that = this.clone();
            Box::pin(async move {
                that.update().await;
            })
        })
        .expect("Failed to create Ethereum TxLog updater");

        self.scheduler
            .add(job)
            .await
            .expect("Failed to add Ethereum TxLog updater to scheduler");
        self.scheduler
            .start()
            .await
            .expect("Failed to start Ethereum TxLog updater scheduler");

        self.ws_block_listener().await;

        info!("Started Ethereum TxLog Updater.");
        // tokio::signal::ctrl_c().await.unwrap(); // Notice: It's will keep the program running.
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_http_block_poller() {
        todo!()
    }
}
