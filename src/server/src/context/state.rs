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

use crate::{
    cache::{memory::StringMemoryCache, redis::StringRedisCache, CacheContainer},
    config::config::{AppConfig, AppDBProperties, AppDBType},
    mgmt::health::{MongoChecker, RedisClusterChecker, SQLiteChecker},
    modules::{
        ethereum::store::{
            ethereum_checkpoint_mongo::EthereumCheckpointMongoRepository,
            ethereum_checkpoint_postgresql::EthereumCheckpointPostgresRepository,
            ethereum_checkpoint_sqlite::EthereumCheckpointSQLiteRepository,
            ethereum_event_mongo::EthereumEventMongoRepository,
            ethereum_event_postgresql::EthereumEventPostgresRepository,
            ethereum_event_sqlite::EthereumEventSQLiteRepository,
        },
        llm::handler::llm_base::{ILLMHandler, LLMManager},
    },
    store::RepositoryContainer,
    sys::store::{
        users_mongo::UserMongoRepository, users_postgresql::UserPostgresRepository, users_sqlite::UserSQLiteRepository,
    },
};
use linkportal_types::{
    modules::ethereum::{ethereum_checkpoint::EthEventCheckpoint, ethereum_event::EthTransactionEvent},
    sys::user::User,
};
use linkportal_utils::httpclients;
use oauth2::basic::BasicClient;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Clone)]
pub struct LinkPortalState {
    pub config: Arc<AppConfig>,
    // The Basic operators.
    pub string_cache: Arc<CacheContainer<String>>,
    pub oidc_client: Option<Arc<openidconnect::core::CoreClient>>,
    pub github_client: Option<Arc<BasicClient>>,
    pub default_http_client: Arc<reqwest::Client>,
    // The Health checker.
    pub sqlite_checker: SQLiteChecker,
    pub mongo_checker: MongoChecker,
    pub redis_cluster_checker: RedisClusterChecker,
    // The System Module repositories.
    pub user_repo: Arc<Mutex<RepositoryContainer<User>>>,
    // The Service Module repositories.
    pub llm_handler: Arc<dyn ILLMHandler + Send + Sync>,
    pub eth_event_repo: Arc<RwLock<RepositoryContainer<EthTransactionEvent>>>,
    pub eth_checkpoint_repo: Arc<RwLock<RepositoryContainer<EthEventCheckpoint>>>,
}

impl LinkPortalState {
    pub async fn new(config: &Arc<AppConfig>) -> Self {
        let cache_config = &config.cache;

        // Build cacher.
        let cache_container = CacheContainer::new(
            Box::new(StringMemoryCache::new(&cache_config.memory)),
            Box::new(StringRedisCache::new(&cache_config.redis)),
        );

        // Build auth clients.
        let auth_clients = (
            crate::util::oidcs::create_oidc_client(&config.auth.oidc)
                .await
                .map(|client| Arc::new(client)),
            crate::util::oauth2::create_oauth2_client(&config.auth.github)
                .await
                .map(|client| Arc::new(client)),
        );

        // Build Tooling http client.
        let http_client = httpclients::build_default();

        // Build App DB repositories.
        let db_config = &config.appdb;
        let user_repo = RepositoryContainer::new(
            match db_config.db_type {
                AppDBType::SQLITE => Some(Box::new(UserSQLiteRepository::new(&db_config.sqlite).await.unwrap())),
                _ => None,
            },
            match db_config.db_type {
                AppDBType::POSTGRESQL => Some(Box::new(
                    UserPostgresRepository::new(&db_config.postgres).await.unwrap(),
                )),
                _ => None,
            },
            match db_config.db_type {
                AppDBType::MONGODB => Some(Box::new(UserMongoRepository::new(&db_config.mongodb).await.unwrap())),
                _ => None,
            },
        );

        let app_state = LinkPortalState {
            // Notice: Arc object clone only increments the reference counter, and does not copy the actual data block.
            config: config.clone(),
            // The basic operators.
            string_cache: Arc::new(cache_container),
            oidc_client: auth_clients.0,
            github_client: auth_clients.1,
            default_http_client: Arc::new(http_client),
            // The Health checker.
            sqlite_checker: SQLiteChecker::new(),
            mongo_checker: MongoChecker::new(),
            redis_cluster_checker: RedisClusterChecker::new(),
            // The System Module repositories.
            user_repo: Arc::new(Mutex::new(user_repo)),
            // The Service Module repositories.
            llm_handler: LLMManager::get_default_implementation(),
            eth_event_repo: Arc::new(RwLock::new(Self::new_eth_event_repo(db_config).await)),
            eth_checkpoint_repo: Arc::new(RwLock::new(Self::new_eth_checkpoint_repo(db_config).await)),
        };

        // Build DI container.
        // let mut di_container = syrette::DIContainer::new();
        // di_container.bind::<dyn IUserHandler>().to::<UserHandler>()?;

        app_state
    }

    pub async fn new_eth_event_repo(db_config: &AppDBProperties) -> RepositoryContainer<EthTransactionEvent> {
        RepositoryContainer::new(
            match db_config.db_type {
                AppDBType::SQLITE => Some(Box::new(
                    EthereumEventSQLiteRepository::new(&db_config.sqlite).await.unwrap(),
                )),
                _ => None,
            },
            match db_config.db_type {
                AppDBType::POSTGRESQL => Some(Box::new(
                    EthereumEventPostgresRepository::new(&db_config.postgres).await.unwrap(),
                )),
                _ => None,
            },
            match db_config.db_type {
                AppDBType::MONGODB => Some(Box::new(
                    EthereumEventMongoRepository::new(&db_config.mongodb).await.unwrap(),
                )),
                _ => None,
            },
        )
    }

    pub async fn new_eth_checkpoint_repo(db_config: &AppDBProperties) -> RepositoryContainer<EthEventCheckpoint> {
        RepositoryContainer::new(
            match db_config.db_type {
                AppDBType::SQLITE => Some(Box::new(
                    EthereumCheckpointSQLiteRepository::new(&db_config.sqlite)
                        .await
                        .unwrap(),
                )),
                _ => None,
            },
            match db_config.db_type {
                AppDBType::POSTGRESQL => Some(Box::new(
                    EthereumCheckpointPostgresRepository::new(&db_config.postgres)
                        .await
                        .unwrap(),
                )),
                _ => None,
            },
            match db_config.db_type {
                AppDBType::MONGODB => Some(Box::new(
                    EthereumCheckpointMongoRepository::new(&db_config.mongodb)
                        .await
                        .unwrap(),
                )),
                _ => None,
            },
        )
    }
}
