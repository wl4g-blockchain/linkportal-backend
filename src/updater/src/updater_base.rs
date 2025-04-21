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

use crate::updater_eth_txlog::EthereumTxLogUpdater;
use anyhow::Error;
use async_trait::async_trait;
use common_telemetry::info;
use lazy_static::lazy_static;
use linkportal_server::config::config;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[async_trait]
pub trait IChainTxLogUpdater: Send + Sync {
    async fn init(&self);
}

lazy_static! {
    static ref SINGLE_INSTANCE: RwLock<ChainTxLogUpdaterManager> = RwLock::new(ChainTxLogUpdaterManager::new());
}

pub struct ChainTxLogUpdaterManager {
    pub implementations: HashMap<String, Arc<dyn IChainTxLogUpdater + Send + Sync>>,
}

impl ChainTxLogUpdaterManager {
    fn new() -> Self {
        ChainTxLogUpdaterManager {
            implementations: HashMap::new(),
        }
    }

    pub fn get() -> &'static RwLock<ChainTxLogUpdaterManager> {
        &SINGLE_INSTANCE
    }

    pub async fn init() {
        info!("Register All chain TxLog updaters ...");

        if let Some(updaters) = &config::get_config().services.updaters {
            for updater_config in updaters {
                if !updater_config.enabled {
                    info!("Skipping implementation chain TxLog updater: {}", updater_config.name);
                    continue;
                }
                // TODO: Full use similar java spi provider mechanism.
                if updater_config.kind == EthereumTxLogUpdater::KIND {
                    match Self::get()
                        .write() // If acquire fails, then it block until acquired.
                        .unwrap() // If acquire fails, then it should panic.
                        .register(
                            updater_config.kind.to_owned(),
                            EthereumTxLogUpdater::new(config::get_config().to_owned(), updater_config).await,
                        ) {
                        Ok(registered) => {
                            info!("Initializing LinkPortal Updater ...");
                            let _ = registered.init().await;
                        }
                        Err(e) => panic!("Failed to register LinkPortal Updater: {}", e),
                    }
                }
            }
        }
    }

    fn register<T: IChainTxLogUpdater + Send + Sync + 'static>(
        &mut self,
        name: String,
        handler: Arc<T>,
    ) -> Result<Arc<T>, Error> {
        if self.implementations.contains_key(&name) {
            tracing::debug!("Already register the Updater '{}'", name);
            return Ok(handler);
        }
        self.implementations.insert(name, handler.to_owned());
        Ok(handler)
    }

    pub async fn get_implementation(name: String) -> Result<Arc<dyn IChainTxLogUpdater + Send + Sync>, Error> {
        // If the read lock is poisoned, the program will panic.
        let this = ChainTxLogUpdaterManager::get().read().unwrap();
        if let Some(implementation) = this.implementations.get(&name) {
            Ok(implementation.to_owned())
        } else {
            let errmsg = format!("Could not obtain registered TxLog Updater '{}'.", name);
            return Err(Error::msg(errmsg));
        }
    }
}
