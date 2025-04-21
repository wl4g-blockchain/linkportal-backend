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
use anyhow::Ok;
use async_trait::async_trait;
use common_telemetry::{error, info, warn};
use ethers::{
    abi::{Abi, RawLog},
    providers::{Http, Middleware, Provider, StreamExt, Ws},
    types::{Log, TransactionReceipt, U64},
};
use futures::future::join_all;
use linkportal_server::{
    config::config::{AppConfig, UpdaterProperties},
    context::state::LinkPortalState,
    store::RepositoryContainer,
};
use linkportal_types::modules::ethereum::ethereum_event::{EthContractSpec, EthTransactionEvent};
use linkportal_types::{modules::ethereum::ethereum_checkpoint::EthEventCheckpoint, BaseBean};
use serde_json::{json, Value};
use std::{fs::File, io::BufReader, sync::Arc};
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};

#[derive(Clone)]
pub struct EthereumTxLogUpdater {
    config: Arc<AppConfig>,
    updater_config: Arc<UpdaterProperties>,
    scheduler: Arc<JobScheduler>,
    contract_specs: Arc<Vec<EthContractSpec>>,
    eth_event_repo: Arc<RwLock<RepositoryContainer<EthTransactionEvent>>>,
    eth_checkpoint_repo: Arc<RwLock<RepositoryContainer<EthEventCheckpoint>>>,
}

impl EthereumTxLogUpdater {
    pub const KIND: &'static str = "ETHEREUM";

    pub async fn new(config: Arc<AppConfig>, updater_config: &UpdaterProperties) -> Arc<Self> {
        let chain_config = updater_config.chain.to_owned();

        let abi: Abi = serde_json::from_reader(BufReader::new(
            File::open(&chain_config.abi_path)
                .expect(format!("Failed to read ABI file: {}", &chain_config.abi_path).as_str()),
        ))
        .expect("Failed to read ABI from JSON");

        let contract_specs = chain_config
            .contract_addres
            .iter()
            .map(|addr| EthContractSpec {
                address: addr.parse().unwrap(),
                abi: abi.to_owned(),
                filter_events: chain_config.filters.to_owned().unwrap_or_default(),
            })
            .collect();

        // Create the this Ethereum compatible TxLog updater instance.
        Arc::new(Self {
            config: config.to_owned(),
            updater_config: Arc::new(updater_config.to_owned()),
            scheduler: Arc::new(
                JobScheduler::new_with_channel_size(updater_config.channel_size)
                    .await
                    .unwrap(),
            ),
            contract_specs: Arc::new(contract_specs),
            eth_event_repo: Arc::new(RwLock::new(LinkPortalState::new_eth_event_repo(&config.appdb).await)),
            eth_checkpoint_repo: Arc::new(RwLock::new(
                LinkPortalState::new_eth_checkpoint_repo(&config.appdb).await,
            )),
        })
    }

    pub(super) async fn update(&self) {
        info!("Updating Ethereum compatible chain TxLog ...");

        let provider_http = Arc::new(
            Provider::<Http>::try_from(&self.updater_config.chain.http_rpc_url)
                .expect("Failed to create HTTP provider"),
        );

        // Load the last persist checkpoint.
        // TODO: Remove this comments.
        // let last_block = self.load_checkpoint().await.expect("Failed to load checkpoint");
        // if let Err(e) = self.http_block_poller(provider_http.clone(), last_block).await {
        //     error!("Failed to http poll update block TxLog: {:?}", e);
        // }
    }

    async fn load_checkpoint(&self) -> anyhow::Result<u64> {
        let eth_checkpoint_repo = self.eth_checkpoint_repo.read().await;
        let checkpoint = eth_checkpoint_repo
            .get(&self.config)
            .select_by_id(1) // TODO: parameter id ?
            .await;
        // if there are no checkpoint, set the last_processed_block to 0.
        match checkpoint {
            std::result::Result::Ok(checkpoint) => Ok(checkpoint.last_processed_block as u64),
            Err(_) => Ok(0),
        }
    }

    async fn ws_block_listener(&self) {
        // Initialize the Ethereum RPC providers.
        let provider_ws = Arc::new(
            Provider::<Ws>::connect(self.updater_config.chain.ws_rpc_url.as_str())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create WebSocket provider: {}", e))
                .expect("Failed to create WebSocket provider"),
        );

        // Load the last persist checkpoint.
        let last_block = self.load_checkpoint().await.expect("Failed to load checkpoint");

        // Start up the WS block listener
        let mut block_stream = provider_ws
            .subscribe_blocks()
            .await
            .expect("Failed to subscribe to blocks with WebSocket");

        while let Some(block) = block_stream.next().await {
            if let Some(block_number) = block.number {
                // Skip the blocks of already been processed.
                if block_number.as_u64() <= last_block {
                    continue;
                }
                info!("Processing the subscribe eth block number: {}", block_number);

                // Fetch transaction receipts for the block.
                let receipts = self
                    .fetch_block_receipts_with_ws(provider_ws.to_owned(), block_number)
                    .await
                    .expect("Failed to fetch block receipts with WebSocket");

                match self.handle_block_recipts(receipts, last_block).await {
                    anyhow::Result::Ok(_) => match self.save_checkpoint(last_block).await {
                        anyhow::Result::Ok(_) => {
                            self.save_checkpoint(last_block)
                                .await
                                .expect("Failed to save checkpoint");
                        }
                        Err(e) => {
                            error!("Error processing block {}: {:?}", last_block, e);
                            continue;
                        }
                    },
                    Err(e) => {
                        error!("Error processing block {}: {:?}", last_block, e);
                        continue;
                    }
                }
            }
        }
    }

    async fn http_block_poller(&self, provider: Arc<Provider<Http>>, mut last_block: u64) -> anyhow::Result<()> {
        let current_block = provider.get_block_number().await?.as_u64();

        // Handle the missing blocks
        while last_block < current_block {
            last_block += 1;
            info!("Processing the missing eth block number: {}", last_block);

            // Fetch transaction receipts for the block.
            let receipts = self
                .fetch_block_receipts_with_http(provider.clone(), last_block.into())
                .await?;

            match self.handle_block_recipts(receipts, last_block).await {
                anyhow::Result::Ok(_) => match self.save_checkpoint(last_block).await {
                    anyhow::Result::Ok(_) => {
                        self.save_checkpoint(last_block).await?;
                    }
                    Err(e) => {
                        error!("Error processing block {}: {:?}", last_block, e);
                        break;
                    }
                },
                Err(e) => {
                    error!("Error processing block {}: {:?}", last_block, e);
                    break;
                }
            }
        }

        todo!()
    }

    /// Obtain the tx receipts in block with WebSocket
    async fn fetch_block_receipts_with_ws(
        &self,
        provider: Arc<Provider<Ws>>,
        block_number: U64,
    ) -> anyhow::Result<Vec<TransactionReceipt>> {
        let block = provider
            .get_block_with_txs(block_number)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Block not found"))?;

        let receipt_futures = block
            .transactions
            .into_iter()
            .map(|tx| provider.get_transaction_receipt(tx.hash));

        let receipts = join_all(receipt_futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();

        Ok(receipts)
    }

    /// Obtain the tx receipts in block with HTTP
    async fn fetch_block_receipts_with_http(
        &self,
        provider: Arc<Provider<Http>>,
        block_number: U64,
    ) -> anyhow::Result<Vec<TransactionReceipt>> {
        let block = provider
            .get_block_with_txs(block_number)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Block not found"))?;

        let receipt_futures = block
            .transactions
            .into_iter()
            .map(|tx| provider.get_transaction_receipt(tx.hash));

        let receipts = join_all(receipt_futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();

        Ok(receipts)
    }

    async fn handle_block_recipts(
        &self,
        receipts: Vec<TransactionReceipt>,
        block_number: u64,
    ) -> anyhow::Result<(), anyhow::Error> {
        // Handle the logs in the transaction receipts.
        let mut events = Vec::new();
        for receipt in receipts {
            for log in receipt.logs {
                if let Some(event) = self.parse_log_as_event(log.to_owned()).await {
                    events.push(event);
                }
            }
        }

        // Persist the All event to DB.
        if !events.is_empty() {
            match self.save_events_batch(events).await {
                anyhow::Result::Ok(_) => match self.save_checkpoint(block_number).await {
                    anyhow::Result::Ok(_) => {
                        self.save_checkpoint(block_number).await?;
                    }
                    Err(e) => {
                        error!("Error process persist with block {}: {:?}", block_number, e);
                        return Err(e.into());
                    }
                },
                Err(e) => {
                    error!("Error process persist with block {}: {:?}", block_number, e);
                    return Err(e.into());
                }
            }
        }

        // Handle the chain transaction rollback.
        match self.handle_rollback(block_number).await {
            anyhow::Result::Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn parse_log_as_event(&self, log: Log) -> Option<EthTransactionEvent> {
        // 查找匹配的合约配置
        let contract_config = self.contract_specs.iter().find(|c| c.address == log.address)?;
        // TODO: Assumption matched it.
        // let contract_config = contracts.get(0)?;

        // 从ABI创建事件解析器
        let abi_event = contract_config.abi.events().find(|e| e.signature() == log.topics[0])?;

        // 检查是否是我们跟踪的事件
        if !contract_config.filter_events.contains(&abi_event.name) {
            return None;
        }

        // 解析原始日志
        let raw_log = RawLog {
            topics: log.topics,
            data: log.data.to_vec(),
        };

        // 将日志解析为动态JSON值
        let abi_log = match abi_event.parse_log(raw_log) {
            anyhow::Result::Ok(log) => log,
            Err(e) => {
                error!("Failed to parse log: {:?}", e);
                return None;
            }
        };

        // 将事件参数转换为键值对
        let mut event_map = serde_json::Map::new();
        for (event_param, log_param) in abi_event.inputs.iter().zip(abi_log.params) {
            let value = match serde_json::to_value(&log_param.value) {
                anyhow::Result::Ok(v) => v,
                Err(_) => json!(format!("{:?}", &log_param.value)),
            };
            event_map.insert(event_param.name.to_owned(), value);
        }

        Some(EthTransactionEvent {
            base: BaseBean::new_default(None),
            block_number: log.block_number.unwrap().as_u64(),
            transaction_hash: format!("{:?}", log.transaction_hash.unwrap()),
            contract_address: format!("{:?}", log.address),
            event_name: abi_event.name.clone(),
            event_data: Value::Object(event_map),
        })
    }

    async fn save_events_batch(&self, events: Vec<EthTransactionEvent>) -> anyhow::Result<()> {
        todo!()
    }

    async fn handle_rollback(&self, new_block_number: u64) -> anyhow::Result<()> {
        // 删除比新区块号更大的所有区块数据
        // let deleted = sqlx::query("DELETE FROM contract_events WHERE block_number > $1")
        //     .bind(new_block_number as i64)
        //     .execute(pool)
        //     .await?;
        // if deleted.rows_affected() > 0 {
        //     info!(
        //         "Reorg detected. Removed {} events from reorged blocks",
        //         deleted.rows_affected()
        //     );
        // }
        // Ok(())
        todo!()
    }

    async fn save_checkpoint(&self, block_number: u64) -> anyhow::Result<()> {
        // sqlx::query(
        //     "INSERT INTO checkpoints (id, last_processed_block)
        //      VALUES (1, $1)
        //      ON CONFLICT (id) DO UPDATE SET last_processed_block = $1",
        // )
        // .bind(block_number as i64)
        // .execute(pool)
        // .await?;
        // Ok(())

        // TODO: build ethereum event object.
        let mut param = EthTransactionEvent::default();
        param.block_number = block_number;
        self.eth_event_repo
            .write()
            .await
            .get(&self.config)
            .insert(param)
            .await?;
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
                "0/30 * * * * *" // every half minute
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
