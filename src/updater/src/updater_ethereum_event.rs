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
use super::updater_base::ILinkPortalUpdater;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use common_telemetry::{error, info, warn};
use ethers::{
    abi::{ethabi, Event, RawLog},
    providers::{Http, Middleware, Provider},
    types::{Address, BlockNumber, Filter, Log, U256},
    utils::hex,
};
use linkportal_server::{config::config::UpdaterProperties, llm::handler::llm_base::LLMManager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{collections::HashMap, str::FromStr};
use tokio_cron_scheduler::{Job, JobScheduler};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EventType {
    // NFT related events
    NFTMint,     // Minting NFT representing RWA
    NFTTransfer, // Transfer of NFT

    // Auction related events
    EnglishAuctionCreated,
    EnglishAuctionBid,
    EnglishAuctionFinalized,
    DutchAuctionCreated,
    DutchAuctionPurchase,
    DutchAuctionFinalized,

    // Token swap related events
    UniswapSwap, // Swapping tokens on Uniswap

    // AAVE related events
    AAVEDeposit,  // Depositing into AAVE for staking
    AAVEWithdraw, // Withdrawing from AAVE
    AAVEBorrow,   // Borrowing from AAVE with collateral
    AAVERepay,    // Repaying AAVE loan

    // StandalonePoolManager events
    PoolDeposit,     // Depositing into LinkPortal's own DeFi pool
    PoolWithdraw,    // Withdrawing from LinkPortal's pool
    PoolBorrow,      // Borrowing from LinkPortal's pool
    PoolRepay,       // Repaying loans from LinkPortal's pool
    PoolLiquidation, // Liquidation events in LinkPortal's pool
    PoolRateChange,  // Changes in interest rates

    // Oracle events
    OracleUpdate, // Price updates from Chainlink oracle

    // Other events
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkPortalEthereumEvent {
    // Event metadata
    pub event_type: EventType,
    pub tx_hash: String,
    pub block_number: u64,
    pub block_hash: String,
    pub log_index: u32,
    pub contract_address: String,
    pub timestamp: DateTime<Utc>,

    // Transaction metadata
    pub from_address: String,
    pub to_address: Option<String>,
    pub gas_used: u64,
    pub gas_price: U256,

    // Event specific data (stored as JSON for flexibility)
    pub event_data: serde_json::Value,

    // For tracking purposes
    pub processed: bool,
    pub processing_timestamp: Option<DateTime<Utc>>,

    // For rollback detection
    pub chain_id: u64,
    pub confirmed: bool,
    pub confirmation_blocks: u32,
}

impl LinkPortalEthereumEvent {
    pub fn new(
        event_type: EventType,
        tx_hash: String,
        block_number: u64,
        block_hash: String,
        log_index: u32,
        contract_address: String,
        from_address: String,
        to_address: Option<String>,
        gas_used: u64,
        gas_price: U256,
        event_data: serde_json::Value,
        chain_id: u64,
    ) -> Self {
        Self {
            event_type,
            tx_hash,
            block_number,
            block_hash,
            log_index,
            contract_address,
            timestamp: Utc::now(), // This should ideally be taken from block timestamp
            from_address,
            to_address,
            gas_used,
            gas_price,
            event_data,
            processed: false,
            processing_timestamp: None,
            chain_id,
            confirmed: false,
            confirmation_blocks: 0,
        }
    }

    pub fn mark_processed(&mut self) {
        self.processed = true;
        self.processing_timestamp = Some(Utc::now());
    }

    pub fn mark_confirmed(&mut self, confirmation_blocks: u32) {
        self.confirmation_blocks = confirmation_blocks;
        self.confirmed = true;
    }

    // Helper methods to access specific data from the event_data field
    pub fn get_token_id(&self) -> Option<U256> {
        self.event_data
            .get("tokenId")
            .and_then(|v| v.as_str())
            .and_then(|s| U256::from_dec_str(s).ok())
    }

    pub fn get_asset_value(&self) -> Option<U256> {
        self.event_data
            .get("value")
            .and_then(|v| v.as_str())
            .and_then(|s| U256::from_dec_str(s).ok())
    }

    // Additional helper methods based on event type
    pub fn is_liquidation_candidate(&self) -> bool {
        // Logic to determine if this event indicates a position that may be liquidated soon
        // This would be used by your AI prediction system
        match self.event_type {
            EventType::PoolBorrow => {
                // Check if collateral ratio is approaching liquidation threshold
                if let Some(ratio) = self.event_data.get("collateralRatio").and_then(|v| v.as_f64()) {
                    // Example threshold - would be based on your system's parameters
                    return ratio < 1.25;
                }
                false
            }
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct EthereumEventUpdater {
    config: UpdaterProperties,
    scheduler: Arc<JobScheduler>,
}

impl EthereumEventUpdater {
    pub const KIND: &'static str = "ETH_EVENT";

    pub async fn new(config: &UpdaterProperties) -> Arc<Self> {
        // Create the this updater handler instance.
        Arc::new(Self {
            config: config.to_owned(),
            scheduler: Arc::new(JobScheduler::new_with_channel_size(config.channel_size).await.unwrap()),
        })
    }

    pub(super) async fn update(&self) {
        info!("Updating Ethereum Events ...");

        // TODO: Unified create the llm handler instance with 'server/src/context/state.rs#llm_handler'
        let llm_handler = LLMManager::get_default_implementation();

        let prompt = "TODO".to_owned();
        match llm_handler.generate(prompt).await {
            Ok(result) => {
                info!("Generated by LLM: {}", result);
                // TODO: continue anthoer processing ...
            }
            Err(e) => {
                error!("Failed to generate rules: {}", e);
                return;
            }
        }
    }

    #[allow(unused)]
    async fn fetch_events(
        &self,
        from_block: BlockNumber,
        to_block: BlockNumber,
    ) -> anyhow::Result<Vec<LinkPortalEthereumEvent>, anyhow::Error> {
        // Initialize Ethereum provider.
        let provider = Provider::<Http>::try_from(self.config.ethereum.rpc_url.as_str())
            .context("Failed to create Ethereum provider")?;

        // Create filters for each contract we want to monitor.
        let mut all_events = Vec::new();

        // for contract_address in contracts {
        let estate_token_addr = Address::from_str(&self.config.ethereum.estate_token_addr)?;

        // Create filter for this contract's events
        let filter = Filter::new()
            .address(estate_token_addr)
            .from_block(from_block)
            .to_block(to_block);

        // Get transaction logs
        let logs = provider.get_logs(&filter).await?;

        // Process logs into our event type
        for log in logs {
            let event = self.parse_ethereum_event(log).await?;
            all_events.push(event);
        }
        // }

        // Store events in database
        for event in &all_events {
            // db_store.store_event(event).await?;
        }

        // Check for and handle potential rollbacks
        // self.check_for_rollbacks(from_block, to_block).await?;

        Ok(all_events)
    }

    // Helper to parse Ethereum logs into our event structure
    async fn parse_ethereum_event(&self, log: Log) -> anyhow::Result<LinkPortalEthereumEvent, anyhow::Error> {
        // Get transaction data for this log
        let tx_hash = log
            .transaction_hash
            .ok_or_else(|| anyhow!("Log missing transaction hash"))?;
        let tx_hash_str = format!("{:x}", tx_hash);

        // Load transaction data to get more context
        let provider =
            Provider::<Http>::try_from(&self.config.ethereum.rpc_url).context("Failed to create Ethereum provider")?;

        let tx = provider
            .get_transaction(tx_hash)
            .await
            .context("Failed to fetch transaction")?
            .ok_or_else(|| anyhow!("Transaction not found"))?;

        let block = provider
            .get_block(log.block_hash.unwrap())
            .await
            .context("Failed to fetch block")?
            .ok_or_else(|| anyhow!("Block not found"))?;

        let timestamp =
            chrono::DateTime::from_timestamp(block.timestamp.as_u64() as i64, 0).unwrap_or_else(|| Utc::now());

        // Get contract type based on address
        let contract_address = format!("{:x}", log.address);
        let event_type = self.determine_event_type(&log)?;

        // Create RawLog for decoding
        let raw_log = RawLog {
            topics: log.topics.clone(),
            data: log.data.0.to_vec(),
        };

        // Decode event data based on event type
        let event_data = match event_type {
            EventType::NFTMint => {
                // Assuming an event signature like "Mint(address,uint256,uint256)"
                let event = self.get_event_signature("NFTMint")?;
                let decoded = event.parse_log(raw_log).context("Failed to decode NFTMint event")?;

                let mut data = serde_json::Map::new();
                for param in decoded.params {
                    match param.name.as_str() {
                        "to" => {
                            data.insert(
                                "to".to_string(),
                                serde_json::Value::String(format!("{:x}", param.value.into_address().unwrap())),
                            );
                        }
                        "tokenId" => {
                            data.insert(
                                "tokenId".to_string(),
                                serde_json::Value::String(param.value.into_uint().unwrap().to_string()),
                            );
                        }
                        "value" => {
                            data.insert(
                                "value".to_string(),
                                serde_json::Value::String(param.value.into_uint().unwrap().to_string()),
                            );
                        }
                        _ => {}
                    }
                }
                serde_json::Value::Object(data)
            }
            EventType::PoolBorrow => {
                // TODO: Assuming an event signature like "Borrow(address,uint256,uint256,uint256)"
                let event = self.get_event_signature("PoolBorrow")?;
                let decoded = event.parse_log(raw_log).context("Failed to decode PoolBorrow event")?;

                let mut data = serde_json::Map::new();
                for param in decoded.params {
                    match param.name.as_str() {
                        "borrower" => {
                            data.insert(
                                "borrower".to_string(),
                                serde_json::Value::String(format!("{:x}", param.value.into_address().unwrap())),
                            );
                        }
                        "amount" => {
                            data.insert(
                                "amount".to_string(),
                                serde_json::Value::String(param.value.into_uint().unwrap().to_string()),
                            );
                        }
                        "collateralAmount" => {
                            let collateral = param.value.into_uint().unwrap();
                            data.insert(
                                "collateralAmount".to_string(),
                                serde_json::Value::String(collateral.to_string()),
                            );

                            // Calculate and add collateral ratio if both values are available
                            if let Some(amount_str) = data.get("amount").and_then(|v| v.as_str()) {
                                if let (Ok(amount), collateral) = (U256::from_dec_str(amount_str), collateral) {
                                    if !amount.is_zero() {
                                        let ratio = collateral.as_u128() as f64 / amount.as_u128() as f64;
                                        data.insert(
                                            "collateralRatio".to_string(),
                                            serde_json::Value::Number(serde_json::Number::from_f64(ratio).unwrap()),
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                serde_json::Value::Object(data)
            }
            // Add handlers for other event types...
            _ => {
                // Generic handler for unknown events
                let mut data = serde_json::Map::new();
                data.insert(
                    "rawData".to_string(),
                    serde_json::Value::String(hex::encode(&log.data.0)),
                );
                data.insert(
                    "topics".to_string(),
                    serde_json::Value::Array(
                        log.topics
                            .iter()
                            .map(|t| serde_json::Value::String(format!("{:x}", t)))
                            .collect(),
                    ),
                );
                serde_json::Value::Object(data)
            }
        };

        // Build the LinkPortalEthereumEvent
        let event = LinkPortalEthereumEvent::new(
            event_type,
            tx_hash_str,
            log.block_number.unwrap_or_default().as_u64(),
            format!("{:x}", log.block_hash.unwrap_or_default()),
            log.log_index.unwrap_or_default().as_u32(),
            contract_address,
            format!("{}", tx.from.to_string()),
            tx.to.map(|addr| format!("{:x}", addr)),
            tx.gas.as_u64(),
            tx.gas_price.unwrap_or_default(),
            event_data,
            self.config.ethereum.chain_id,
        );

        Ok(event)
    }
    // Get ABI event definition by name
    fn get_event_signature(&self, event_name: &str) -> anyhow::Result<Event, anyhow::Error> {
        // This would typically load from a cache of contract ABIs
        // For simplicity, we're hardcoding a few event signatures
        match event_name {
            "NFTMint" => {
                let event = Event {
                    name: "Mint".to_string(),
                    inputs: vec![
                        ethabi::EventParam {
                            name: "to".to_string(),
                            kind: ethabi::ParamType::Address,
                            indexed: false,
                        }, // to address
                        ethabi::EventParam {
                            name: "tokenId".to_string(),
                            kind: ethabi::ParamType::Uint(256),
                            indexed: false,
                        }, // tokenId
                        ethabi::EventParam {
                            name: "value".to_string(),
                            kind: ethabi::ParamType::Uint(256),
                            indexed: false,
                        }, // value
                    ],
                    anonymous: false,
                };
                Ok(event)
            }
            "PoolBorrow" => {
                let event = Event {
                    name: "Borrow".to_string(),
                    inputs: vec![
                        ethabi::EventParam {
                            name: "borrower".to_string(),
                            kind: ethabi::ParamType::Address,
                            indexed: false,
                        }, // borrower
                        ethabi::EventParam {
                            name: "amount".to_string(),
                            kind: ethabi::ParamType::Uint(256),
                            indexed: false,
                        }, // amount
                        ethabi::EventParam {
                            name: "collateralAmount".to_string(),
                            kind: ethabi::ParamType::Uint(256),
                            indexed: false,
                        }, // collateralAmount
                        ethabi::EventParam {
                            name: "timestamp".to_string(),
                            kind: ethabi::ParamType::Uint(256),
                            indexed: false,
                        }, // timestamp
                    ],
                    anonymous: false,
                };
                Ok(event)
            }
            // Add more event definitions as needed
            _ => Err(anyhow!("Unknown event type: {}", event_name)),
        }
    }

    // Determine event type from the log
    fn determine_event_type(&self, log: &Log) -> anyhow::Result<EventType, anyhow::Error> {
        // In a real implementation, you would:
        // 1. Have a mapping of contract addresses to their types
        // 2. Use the topic[0] (event signature) to determine the specific event

        if log.topics.is_empty() {
            return Ok(EventType::Unknown);
        }

        // The first topic is the event signature
        let event_sig = log.topics[0];

        // TODO: Example implementation - match on known signatures
        // In practice, you would build this from contract ABIs
        match format!("{:x}", event_sig).as_str() {
            // These are example hashes - you'd use your actual event signature hashes
            "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef" => {
                // This is ERC20 Transfer signature
                Ok(EventType::NFTTransfer)
            }
            "0f6798a560793a54c3bcfe86a93cde1e73087d944c0ea20544137d4121396885" => {
                // Fictional Mint event signature
                Ok(EventType::NFTMint)
            }
            "5dac0c1b1112564a045ba9c454e0a0deb49f54a6cbb531162eb9d05115e2b4c8" => {
                // Fictional PoolBorrow event signature
                Ok(EventType::PoolBorrow)
            }
            // Add more signatures as needed
            _ => {
                // If we can't identify the event, default to Unknown
                Ok(EventType::Unknown)
            }
        }
    }

    async fn check_for_rollbacks(&self, from_block: BlockNumber, to_block: BlockNumber) -> anyhow::Result<()> {
        // let db_store = DatabaseStore::from_config(&self.config.database);
        // let provider = Provider::<Http>::try_from(&self.config.ethereum_rpc_url)?;

        // // Strategy:
        // // 1. Get stored events for the block range
        // // 2. For each block, verify it's still in the canonical chain
        // // 3. If not, mark events as unconfirmed and reprocess

        // // Get events from database for this block range
        // let stored_events = db_store
        //     .get_events_by_block_range(
        //         from_block.as_number().unwrap_or_default().as_u64(),
        //         to_block.as_number().unwrap_or_default().as_u64(),
        //     )
        //     .await?;

        // // Group events by block number
        // let mut blocks_to_check = HashMap::new();
        // for event in &stored_events {
        //     blocks_to_check
        //         .entry(event.block_number)
        //         .or_insert_with(|| event.block_hash.clone());
        // }

        // // Check each block's hash against current chain state
        // let mut rollback_blocks = Vec::new();

        // for (block_num, stored_hash) in blocks_to_check {
        //     let current_block = provider
        //         .get_block(block_num)
        //         .await
        //         .context("Failed to fetch block for rollback check")?;

        //     if let Some(block) = current_block {
        //         let current_hash = format!("{:x}", block.hash.unwrap_or_default());
        //         if current_hash != stored_hash {
        //             // This block has been reorganized
        //             rollback_blocks.push(block_num);
        //             warn!(
        //                 "Block {} has been reorganized. Old hash: {}, New hash: {}",
        //                 block_num,
        //                 stored_hash,
        //                 current_hash
        //             );
        //         }
        //     } else {
        //         // Block no longer exists - definite reorg
        //         rollback_blocks.push(block_num);
        //         warn!(
        //             "Block {} no longer exists in chain - reorganization detected",
        //             block_num
        //         );
        //     }
        // }

        // if !rollback_blocks.is_empty() {
        //     warn!(
        //         "Detected chain reorganization affecting {} blocks",
        //         rollback_blocks.len()
        //     );

        //     // Handle the rollback
        //     for block_num in rollback_blocks {
        //         // 1. Mark all events from this block as unconfirmed/invalid
        //         db_store.mark_events_unconfirmed(block_num).await?;

        //         // 2. Queue block for reprocessing
        //         self.queue_block_for_reprocessing(block_num).await?;
        //     }
        // }

        Ok(())
    }

    async fn queue_block_for_reprocessing(&self, block_number: u64) -> anyhow::Result<()> {
        // Add block to a reprocessing queue
        // This could be implemented as a separate table in your database
        // or as a message in a queue system

        // TODO:
        // let db_store = DatabaseStore::from_config(&self.config.database);
        // db_store.add_to_reprocessing_queue(block_number).await?;

        info!(
            "Queued block {} for reprocessing due to chain reorganization",
            block_number
        );
        Ok(())
    }
}

#[async_trait]
impl ILinkPortalUpdater for EthereumEventUpdater {
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
                info!("{:?} Hi I ran", chrono::Utc::now());
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
