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

// use openai::chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole};
use super::updater_base::IChainTxLogUpdater;
use async_trait::async_trait;
use common_telemetry::{error, info, warn};
use ethers::{
    abi::{Abi, RawLog},
    providers::{Http, Middleware, Provider, StreamExt, Ws},
    types::{BlockNumber, Log, TransactionReceipt, H160, U64},
};
use futures::future::join_all;
use linkportal_server::config::config::UpdaterProperties;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::{fs::File, io::BufReader, sync::Arc, time::Duration};
use tokio_cron_scheduler::{Job, JobScheduler};

#[derive(Debug, sqlx::FromRow)]
pub struct SyncCheckpoint {
    last_processed_block: i64,
}

// The DB record for each chain event.
#[derive(Debug)]
pub struct EthereumEventRecord {
    block_number: u64,
    transaction_hash: String,
    contract_address: String,
    event_name: String,
    event_data: Value, // Event JSON data.
}

// The Contract watch configuration
pub struct EthContractSpec {
    address: H160,
    abi: Abi,
    filter_events: Vec<String>, // Target watch chain event names.
}

#[derive(Clone)]
pub struct EthereumEventUpdater {
    config: Arc<UpdaterProperties>,
    scheduler: Arc<JobScheduler>,
}

impl EthereumEventUpdater {
    pub const KIND: &'static str = "ETHEREUM_TX_LOG";
    pub const FILTER_EVENT_NAME: &'static str = "eventName";

    pub async fn new(config: &UpdaterProperties) -> Arc<Self> {
        // Create the this updater handler instance.
        Arc::new(Self {
            config: Arc::new(config.to_owned()),
            scheduler: Arc::new(JobScheduler::new_with_channel_size(config.channel_size).await.unwrap()),
        })
    }

    pub(super) async fn update(&self) {
        info!("Updating Ethereum chain TxLog ...");
        todo!()
    }

    #[allow(unused)]
    async fn fetch_events(
        &self,
        from_block: BlockNumber,
        to_block: BlockNumber,
    ) -> anyhow::Result<Vec<EthereumEventRecord>, anyhow::Error> {
        let this = Arc::new(self.to_owned());

        // TODO: Initialize the database pool.
        let db_pool = PgPool::connect("postgres://postgres:123456@jw-mac-pro.local:35432/linkportal").await?;
        // TODO: Initialize the tables.
        self.init_db(&db_pool).await?;

        // Initialize the Ethereum RPC providers.
        let provider_ws = Arc::new(
            Provider::<Ws>::connect("wss://eth-mainnet.g.alchemy.com/v2/M8QUxbFISVXMqMxvWWKx-N2cxUJF9jmD").await?,
        );
        let provider_http = Arc::new(Provider::<Http>::try_from(
            "https://eth-mainnet.g.alchemy.com/v2/M8QUxbFISVXMqMxvWWKx-N2cxUJF9jmD",
        )?);

        // Load the contract configurations.
        let contract_specs = Arc::new(self.load_contract_specs()?);

        // Load the last persist checkpoint.
        let last_block = self.load_checkpoint(&db_pool).await?;

        // Start up the WS block listener
        let ws_task = {
            let that = Arc::clone(&this);
            let db_pool_ws = db_pool.clone(); // Clone for WS task
            let contracts_ws = Arc::clone(&contract_specs); // Clone for WS task
            tokio::spawn(async move {
                if let Err(e) = that
                    .ws_block_listener(contracts_ws, provider_ws.clone(), db_pool_ws, last_block)
                    .await
                {
                    error!("WS block listener error: {:?}", e);
                }
            })
        };

        // Start up the HTTP block poller for lost persist blocks.
        let poll_task = {
            let that = Arc::clone(&this);
            let contracts_http = Arc::clone(&contract_specs); // Clone for HTTP task
            let db_pool_http = db_pool.clone(); // Clone for HTTP task
            tokio::spawn(async move {
                if let Err(e) = that
                    .http_block_poller(contracts_http, provider_http.clone(), db_pool_http, last_block)
                    .await
                {
                    error!("HTTP block poller error: {:?}", e);
                }
            })
        };

        let result = tokio::try_join!(ws_task, poll_task)?;

        todo!()
    }

    async fn init_db(&self, pool: &PgPool) -> anyhow::Result<()> {
        // Create the contract_events table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS contract_events (
                id SERIAL PRIMARY KEY,
                block_number BIGINT NOT NULL,
                transaction_hash VARCHAR(66) NOT NULL,
                contract_address VARCHAR(42) NOT NULL,
                event_name VARCHAR(255) NOT NULL,
                event_data JSONB NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                UNIQUE (transaction_hash, contract_address, event_name)
            );
            "#,
        )
        .execute(pool)
        .await?;

        // Create the idx_contract_events_block_number index
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_contract_events_block_number ON contract_events (block_number);
            "#,
        )
        .execute(pool)
        .await?;

        // Create the idx_contract_events_contract_address index
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_contract_events_contract_address ON contract_events (contract_address);
            "#,
        )
        .execute(pool)
        .await?;

        // Create the idx_contract_events_event_name index
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_contract_events_event_name ON contract_events (event_name);
            "#,
        )
        .execute(pool)
        .await?;

        // Create the checkpoints table
        sqlx::query(
            r#"
        CREATE TABLE IF NOT EXISTS checkpoints (
            id SERIAL PRIMARY KEY,
            last_processed_block BIGINT NOT NULL
        );
        "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    fn read_abi_from_json(&self, path: &str) -> anyhow::Result<Abi> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let abi: Abi = serde_json::from_reader(reader)?;
        Ok(abi)
    }

    fn load_contract_specs(&self) -> anyhow::Result<Vec<EthContractSpec>, anyhow::Error> {
        let chain_config = self.config.chain.to_owned();

        let contract_specs = chain_config
            .contract_addres
            .iter()
            .map(|addr| EthContractSpec {
                address: addr.parse().unwrap(),
                abi: self
                    .read_abi_from_json(&chain_config.abi_path)
                    .expect("Failed to read ABI from JSON"),
                filter_events: chain_config.filters.to_owned().unwrap_or_default(),
            })
            .collect();

        Ok(contract_specs)
    }

    async fn load_checkpoint(&self, pool: &PgPool) -> anyhow::Result<u64> {
        let checkpoint =
            sqlx::query_as::<_, SyncCheckpoint>("SELECT last_processed_block FROM checkpoints WHERE id = 1")
                .fetch_optional(pool)
                .await?;

        Ok(checkpoint.map(|c| c.last_processed_block as u64).unwrap_or(0))
    }

    async fn ws_block_listener(
        &self,
        contract_specs: Arc<Vec<EthContractSpec>>,
        provider: Arc<Provider<Ws>>,
        db_pool: PgPool,
        last_block: u64,
    ) -> anyhow::Result<()> {
        let mut block_stream = provider.subscribe_blocks().await?;

        while let Some(block) = block_stream.next().await {
            if let Some(block_number) = block.number {
                // Skip the blocks of already been processed.
                if block_number.as_u64() <= last_block {
                    continue;
                }
                info!("Processing the subscribe eth block number: {}", block_number);

                // Fetch transaction receipts for the block.
                let receipts = self
                    .fetch_block_receipts_with_ws(contract_specs.to_owned(), provider.to_owned(), block_number)
                    .await?;

                // Handle the logs in the transaction receipts
                let mut events = Vec::new();
                for receipt in receipts {
                    for log in receipt.logs {
                        if let Some(event) = self.parse_log_as_event(log.to_owned(), contract_specs.to_owned()).await {
                            events.push(event);
                        }
                    }
                }

                // Persist the All event to DB.
                if !events.is_empty() {
                    self.save_events_batch(&db_pool, events).await?;
                }

                // 处理链重组
                self.handle_reorg(&db_pool, block_number.as_u64()).await?;

                // Save events to the database
                self.save_checkpoint(&db_pool, block_number.as_u64()).await?;
            }
        }
        Ok(())
    }

    async fn http_block_poller(
        &self,
        contract_specs: Arc<Vec<EthContractSpec>>,
        provider: Arc<Provider<Http>>,
        db_pool: PgPool,
        mut last_block: u64,
    ) -> anyhow::Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(15));

        loop {
            interval.tick().await;

            let current_block = provider.get_block_number().await?.as_u64();

            // Handle the missing blocks
            while last_block < current_block {
                last_block += 1;
                info!("Processing the missing eth block number: {}", last_block);

                // Fetch transaction receipts for the block.
                let receipts = self
                    .fetch_block_receipts_with_http(contract_specs.to_owned(), provider.clone(), last_block.into())
                    .await?;

                // Handle the logs in the transaction receipts.
                let mut events = Vec::new();
                for receipt in receipts {
                    for log in receipt.logs {
                        if let Some(event) = self.parse_log_as_event(log.clone(), contract_specs.to_owned()).await {
                            events.push(event);
                        }
                    }
                }

                // Persist the All event to DB.
                if !events.is_empty() {
                    match self.save_events_batch(&db_pool, events).await {
                        Ok(_) => match self.save_checkpoint(&db_pool, last_block).await {
                            Ok(_) => {
                                self.save_checkpoint(&db_pool, last_block).await?;
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

                // 处理链重组
                match self.handle_reorg(&db_pool, last_block).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error processing block {}: {:?}", last_block, e);
                        break;
                    }
                }
            }
        }
    }

    // Obtain the tx receipts in block with WebSocket
    async fn fetch_block_receipts_with_ws(
        &self,
        contract_specs: Arc<Vec<EthContractSpec>>,
        provider: Arc<Provider<Ws>>,
        block_number: U64,
    ) -> anyhow::Result<Vec<TransactionReceipt>> {
        // 获取区块中的所有日志
        let logs = provider
            .get_logs(
                &ethers::types::Filter::new()
                    .address(contract_specs.get(0).unwrap().address)
                    .from_block(block_number),
            )
            .await
            .unwrap();

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

    // Obtain the tx receipts in block with HTTP
    async fn fetch_block_receipts_with_http(
        &self,
        contract_specs: Arc<Vec<EthContractSpec>>,
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

    async fn parse_log_as_event(
        &self,
        log: Log,
        contract_specs: Arc<Vec<EthContractSpec>>,
    ) -> Option<EthereumEventRecord> {
        // 查找匹配的合约配置
        let contract_config = contract_specs.iter().find(|c| c.address == log.address)?;
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
            Ok(log) => log,
            Err(e) => {
                error!("Failed to parse log: {:?}", e);
                return None;
            }
        };

        // 将事件参数转换为键值对
        let mut event_map = serde_json::Map::new();
        for (event_param, log_param) in abi_event.inputs.iter().zip(abi_log.params) {
            let value = match serde_json::to_value(&log_param.value) {
                Ok(v) => v,
                Err(_) => json!(format!("{:?}", &log_param.value)),
            };
            event_map.insert(event_param.name.to_owned(), value);
        }

        Some(EthereumEventRecord {
            block_number: log.block_number.unwrap().as_u64(),
            transaction_hash: format!("{:?}", log.transaction_hash.unwrap()),
            contract_address: format!("{:?}", log.address),
            event_name: abi_event.name.clone(),
            event_data: Value::Object(event_map),
        })
    }

    async fn save_events_batch(&self, db_pool: &PgPool, events: Vec<EthereumEventRecord>) -> anyhow::Result<()> {
        let mut tx = db_pool.begin().await?;

        for event in events {
            sqlx::query(
                r#"
                INSERT INTO contract_events 
                (block_number, transaction_hash, contract_address, event_name, event_data)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (transaction_hash, contract_address, event_name) DO NOTHING
                "#,
            )
            .bind(event.block_number as i64)
            .bind(event.transaction_hash)
            .bind(event.contract_address)
            .bind(event.event_name)
            .bind(event.event_data)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    // 处理链重组
    async fn handle_reorg(&self, pool: &PgPool, new_block_number: u64) -> anyhow::Result<()> {
        // 删除比新区块号更大的所有区块数据
        let deleted = sqlx::query("DELETE FROM contract_events WHERE block_number > $1")
            .bind(new_block_number as i64)
            .execute(pool)
            .await?;

        if deleted.rows_affected() > 0 {
            info!(
                "Reorg detected. Removed {} events from reorged blocks",
                deleted.rows_affected()
            );
        }

        Ok(())
    }

    async fn save_checkpoint(&self, pool: &PgPool, block_number: u64) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO checkpoints (id, last_processed_block) 
             VALUES (1, $1)
             ON CONFLICT (id) DO UPDATE SET last_processed_block = $1",
        )
        .bind(block_number as i64)
        .execute(pool)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl IChainTxLogUpdater for EthereumEventUpdater {
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

        info!("Starting Analytics handler with cron '{}'", cron);
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
        // Notice: It's will keep the program running
        // tokio::signal::ctrl_c().await.unwrap();
    }
}

#[cfg(test)]
mod tests {
    // use std::env;
    // use crate::config::config::{ AppConfigProperties, LlmProperties };
    // use super::*;

    // #[tokio::test]
    // async fn test_analyze_with_qwen() {
    //     let mut config = AppConfigProperties::default();

    //     let mut analyze_config = &AnalyticsProperties::default();
    //     analyze_config.kind = SimpleLlmAnalyticsHandler::KIND.to_owned();
    //     analyze_config.name = "defaultAnalyze".to_string();
    //     analyze_config.cron = "0/10 * * * * *".to_string();
    //     config.linkportal.analytics.push(analyze_config);

    //     let mut llm_config = LlmProperties::default();
    //     //llm_config.api_url = "https://api.openai.com/v1/chat/completions".to_string();
    //     llm_config.api_url = "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string();
    //     llm_config.api_key = env::var("TEST_OPENAI_KEY").ok().unwrap();
    //     //llm_config.model = "gpt-3.5-turbo".to_string();
    //     llm_config.model = "qwen-plus".to_string();
    //     config.linkportal.llm = llm_config;

    //     let handler = SimpleLlmAnalyticsHandler::init(analyze_config).await;
    //     handler.analyze().await;
    // }
}
