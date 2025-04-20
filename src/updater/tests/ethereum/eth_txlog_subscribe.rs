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

#[cfg(test)]
pub mod tests {
    use ethers::{
        abi::{Abi, RawLog},
        providers::{Http, Middleware, Provider, StreamExt, Ws},
        types::{Log, TransactionReceipt, H160, U64},
    };
    use futures::future::join_all;
    use serde_json::{json, Value};
    use sqlx::PgPool;
    use std::{sync::Arc, time::Duration};

    #[derive(Debug, sqlx::FromRow)]
    pub struct SyncCheckpoint {
        last_processed_block: i64,
    }

    // The DB record for each chain event.
    #[derive(Debug)]
    pub struct EventRecord {
        block_number: u64,
        transaction_hash: String,
        contract_address: String,
        event_name: String,
        event_data: Value, // Event JSON data.
    }

    // The Contract watch configuration
    pub struct ContractConfig {
        address: H160,
        abi: Abi,
        tracked_events: Vec<String>, // Target watch chain event names.
    }

    #[tokio::test]
    pub async fn robust_event_monitor() -> anyhow::Result<()> {
        // 1. Initialize the database pool.
        let db_pool = PgPool::connect("postgres://postgres:123456@jw-mac-pro.local:35432/linkportal").await?;

        // 1.1 Initialize the tables.
        init_db(&db_pool).await?;

        // 2. Initialize the Ethereum RPC providers.
        let provider_ws = Arc::new(
            Provider::<Ws>::connect("wss://eth-mainnet.g.alchemy.com/v2/M8QUxbFISVXMqMxvWWKx-N2cxUJF9jmD").await?,
        );
        let provider_http = Arc::new(Provider::<Http>::try_from(
            "https://eth-mainnet.g.alchemy.com/v2/M8QUxbFISVXMqMxvWWKx-N2cxUJF9jmD",
        )?);

        // 3. Load the contract configurations.
        let contracts = Arc::new(load_contract_configs()?);

        // 4. Load the last persist checkpoint.
        let last_block = load_checkpoint(&db_pool).await?;

        // 5. Start up the WS block listener
        let ws_task = tokio::spawn(ws_block_listener(
            contracts.to_owned(),
            provider_ws.clone(),
            db_pool.clone(),
            last_block,
        ));

        // 6. Start up the HTTP block poller for lost persist blocks.
        let poll_task = tokio::spawn(http_block_poller(
            contracts.to_owned(),
            provider_http.clone(),
            db_pool.clone(),
            last_block,
        ));

        let result = tokio::try_join!(ws_task, poll_task)?;
        Ok(())
    }

    async fn init_db(pool: &PgPool) -> anyhow::Result<()> {
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

    fn load_contract_configs() -> anyhow::Result<Vec<ContractConfig>> {
        let mut configs = Vec::new();

        // For example: Uniswap V2 factory contract.
        let uniswap_abi: Abi = serde_json::from_slice(include_bytes!("./abi/UniswapV2Factory.json"))?;
        configs.push(ContractConfig {
            address: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".parse()?,
            abi: uniswap_abi,
            tracked_events: vec!["PairCreated".to_string()],
        });

        // Add to more contracts ...
        Ok(configs)
    }

    async fn load_checkpoint(pool: &PgPool) -> anyhow::Result<u64> {
        let checkpoint =
            sqlx::query_as::<_, SyncCheckpoint>("SELECT last_processed_block FROM checkpoints WHERE id = 1")
                .fetch_optional(pool)
                .await?;

        Ok(checkpoint.map(|c| c.last_processed_block as u64).unwrap_or(0))
    }

    async fn ws_block_listener(
        contracts: Arc<Vec<ContractConfig>>,
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

                // Example processing logic (replace with actual implementation)
                println!("Processing block: {}", block_number);

                // Fetch transaction receipts for the block.
                let receipts =
                    fetch_block_receipts_with_ws(contracts.to_owned(), provider.to_owned(), block_number).await?;

                // 处理每个收据中的日志
                let mut events = Vec::new();
                for receipt in receipts {
                    for log in receipt.logs {
                        if let Some(event) = parse_log_as_event(log.to_owned(), contracts.to_owned()).await {
                            events.push(event);
                        }
                    }
                }

                // 批量存储事件
                if !events.is_empty() {
                    save_events_batch(&db_pool, events).await?;
                }

                // 处理链重组
                handle_reorg(&db_pool, block_number.as_u64()).await?;

                // Save events to the database
                save_checkpoint(&db_pool, block_number.as_u64()).await?;
            }
        }
        Ok(())
    }

    async fn http_block_poller(
        contracts: Arc<Vec<ContractConfig>>,
        provider: Arc<Provider<Http>>,
        db_pool: PgPool,
        mut last_block: u64,
    ) -> anyhow::Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(15));

        loop {
            interval.tick().await;

            let current_block = provider.get_block_number().await?.as_u64();

            // 处理遗漏的区块
            while last_block < current_block {
                last_block += 1;

                // Example processing logic (replace with actual implementation)
                println!("Processing block: {}", last_block);

                // Fetch transaction receipts for the block.
                let receipts =
                    fetch_block_receipts_with_http(contracts.to_owned(), provider.clone(), last_block.into()).await?;

                // 处理每个收据中的日志
                let mut events = Vec::new();
                for receipt in receipts {
                    for log in receipt.logs {
                        if let Some(event) = parse_log_as_event(log.clone(), contracts.to_owned()).await {
                            events.push(event);
                        }
                    }
                }

                // 批量存储事件
                if !events.is_empty() {
                    match save_events_batch(&db_pool, events).await {
                        Ok(_) => {
                            // Save events to the database
                            match save_checkpoint(&db_pool, last_block).await {
                                Ok(_) => {
                                    save_checkpoint(&db_pool, last_block).await?;
                                }
                                Err(e) => {
                                    eprintln!("Error processing block {}: {:?}", last_block, e);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error processing block {}: {:?}", last_block, e);
                            break;
                        }
                    }
                }

                // 处理链重组
                match handle_reorg(&db_pool, last_block).await {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error processing block {}: {:?}", last_block, e);
                        break;
                    }
                }
            }
        }
    }

    /// Obtain the receipts in the block with WebSocket.
    async fn fetch_block_receipts_with_ws(
        contracts: Arc<Vec<ContractConfig>>,
        provider: Arc<Provider<Ws>>,
        block_number: U64,
    ) -> anyhow::Result<Vec<TransactionReceipt>> {
        // TODO: Obtain the tx event/logs by filter.
        let logs = provider
            .get_logs(
                &ethers::types::Filter::new()
                    .address(contracts.get(0).unwrap().address)
                    .from_block(block_number),
            )
            .await
            .unwrap();

        let receipts = provider.get_block_receipts(block_number).await?;

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

    // 获取区块中的所有收据 with HTTP
    async fn fetch_block_receipts_with_http(
        contracts: Arc<Vec<ContractConfig>>,
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

    // 通用日志解析为事件
    async fn parse_log_as_event(log: Log, contracts: Arc<Vec<ContractConfig>>) -> Option<EventRecord> {
        // 查找匹配的合约配置
        let contract_config = contracts.iter().find(|c| c.address == log.address)?;
        // TODO: Assumption matched it.
        // let contract_config = contracts.get(0)?;

        // 从ABI创建事件解析器
        let abi_event = contract_config.abi.events().find(|e| e.signature() == log.topics[0])?;

        // 检查是否是我们跟踪的事件
        if !contract_config.tracked_events.contains(&abi_event.name) {
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
                eprintln!("Failed to parse log: {:?}", e);
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

        Some(EventRecord {
            block_number: log.block_number.unwrap().as_u64(),
            transaction_hash: format!("{:?}", log.transaction_hash.unwrap()),
            contract_address: format!("{:?}", log.address),
            event_name: abi_event.name.clone(),
            event_data: Value::Object(event_map),
        })
    }

    // 批量保存事件到数据库
    async fn save_events_batch(db_pool: &PgPool, events: Vec<EventRecord>) -> anyhow::Result<()> {
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
    async fn handle_reorg(pool: &PgPool, new_block_number: u64) -> anyhow::Result<()> {
        // 删除比新区块号更大的所有区块数据
        let deleted = sqlx::query("DELETE FROM contract_events WHERE block_number > $1")
            .bind(new_block_number as i64)
            .execute(pool)
            .await?;

        if deleted.rows_affected() > 0 {
            println!(
                "Reorg detected. Removed {} events from reorged blocks",
                deleted.rows_affected()
            );
        }

        Ok(())
    }

    async fn save_checkpoint(pool: &PgPool, block_number: u64) -> anyhow::Result<()> {
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
