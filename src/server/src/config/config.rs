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

use crate::mgmt::apm::logging::LogMode;
use crate::mgmt::health::HEALTHZ_URI;
use arc_swap::ArcSwap;
use config::Config;
use dotenv::dotenv;
use globset::{Glob, GlobSet, GlobSetBuilder};
use jsonwebtoken::Algorithm;
use lazy_static::lazy_static;
use linkportal_utils::secrets::SecretHelper;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, ops::Deref, str::FromStr, sync::Arc, time::Duration};
use validator::Validate;

// Global program information.
pub const GIT_VERSION: &str = env!("GIT_VERSION");
pub const GIT_COMMIT_HASH: &str = env!("GIT_COMMIT_HASH");
pub const GIT_BUILD_DATE: &str = env!("GIT_BUILD_DATE");

// Global static resources.
pub const DEFAULT_INDEX_HTML: &str = include_str!("../../../../static/index.html");
pub const DEFAULT_LOGIN_HTML: &str = include_str!("../../../../static/login.html");
pub const DEFAULT_404_HTML: &str = include_str!("../../../../static/404.html");
pub const DEFAULT_403_HTML: &str = include_str!("../../../../static/403.html");

lazy_static! {
    pub static ref VERSION: String = format!(
        "GitVersion: {}, GitHash: {}, GitBuildDate: {}",
        env!("GIT_VERSION"),
        env!("GIT_COMMIT_HASH"),
        env!("GIT_BUILD_DATE")
    );
}

//
// App Properties.
//
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
#[serde(rename_all = "kebab-case")]
pub struct AppConfigProperties {
    #[serde(rename = "service-name")]
    #[validate(length(min = 1, max = 32))]
    pub service_name: String,
    #[serde(default = "ServerProperties::default")]
    pub server: ServerProperties,
    #[serde(default = "MgmtProperties::default")]
    pub mgmt: MgmtProperties,
    #[serde(default = "LoggingProperties::default")]
    pub logging: LoggingProperties,
    #[serde(default = "SwaggerProperties::default")]
    pub swagger: SwaggerProperties,
    #[serde(default = "AuthProperties::default")]
    pub auth: AuthProperties,
    #[serde(default = "CacheProperties::default")]
    pub cache: CacheProperties,
    #[serde(default = "AppDBProperties::default")]
    pub appdb: AppDBProperties,
    #[serde(default = "VectorDBProperties::default")]
    pub vecdb: VectorDBProperties,
    #[serde(default = "ServicesProperties::default")]
    pub services: ServicesProperties,
}

//
// Server Properties.
//
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerProperties {
    #[serde(rename = "host")]
    pub host: String,
    #[serde(rename = "port")]
    pub port: u16,
    #[serde(rename = "context-path")]
    pub context_path: Option<String>,
}

//
// Management Properties.
//
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MgmtProperties {
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "host")]
    pub host: String,
    #[serde(rename = "port")]
    pub port: u16,
    #[serde(default = "TokioConsoleProperties::default", rename = "tokio-console")]
    pub tokio_console: TokioConsoleProperties,
    #[serde(default = "PyroscopeAgentProperties::default")]
    pub pyroscope: PyroscopeAgentProperties,
    #[serde(default = "OtelProperties::default")]
    pub otel: OtelProperties,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokioConsoleProperties {
    pub enabled: bool,
    //#[env_config(name = "MW_TOKIO_CONSOLE_SERVER_BIND", default = "0.0.0.0:6699")]
    #[serde(rename = "server-bind")]
    pub server_bind: String,
    pub retention: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PyroscopeAgentProperties {
    pub enabled: bool,
    #[serde(rename = "server-url")]
    pub server_url: String,
    #[serde(rename = "auth-token")]
    pub auth_token: Option<String>,
    pub tags: Option<Vec<(String, String)>>,
    #[serde(rename = "sample-rate")]
    pub sample_rate: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OtelProperties {
    pub enabled: bool,
    pub endpoint: String,
    pub protocol: String,
    pub timeout: Option<u64>,
    // Notice: More OTEL custom configuration use to environment: OTEL_SPAN_xxx, see to: opentelemetry_sdk::trace::config::default()
}

//
// Logging Properties.
//
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingProperties {
    pub mode: LogMode,
    pub level: String,
}

//
// Swagger Properties.
//
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwaggerProperties {
    pub enabled: bool,
    // pub title: String,
    // pub description: String,
    // pub version: String,
    // pub license_name: String,
    // pub license_url: String,
    // pub contact_name: String,
    // pub contact_email: String,
    // pub contact_url: String,
    // pub terms_of_service: String,
    // //pub security_definitions: vec![],
    pub ui_path: String,
    pub openapi_url: String,
}

//
// Auth Properties.
//
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthProperties {
    #[serde(rename = "jwt-ak-name")]
    pub jwt_ak_name: Option<String>,
    #[serde(rename = "jwt-rk-name")]
    pub jwt_rk_name: Option<String>,
    #[serde(rename = "jwt-validity-ak")]
    pub jwt_validity_ak: Option<u64>,
    #[serde(rename = "jwt-validity-rk")]
    pub jwt_validity_rk: Option<u64>,
    #[serde(rename = "jwt-secret")]
    pub jwt_secret: Option<String>,
    #[serde(rename = "jwt-algorithm")]
    pub jwt_algorithm: Option<String>,
    #[serde(rename = "anonymous-paths")]
    pub anonymous_paths: Option<Vec<String>>,
    #[serde(rename = "oidc")]
    pub oidc: OidcProperties,
    #[serde(rename = "github")]
    pub github: GithubProperties,
    #[serde(rename = "login-url")]
    pub login_url: Option<String>,
    #[serde(rename = "success-url")]
    pub success_url: Option<String>,
    #[serde(rename = "unauthz-url")]
    pub unauthz_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OidcProperties {
    pub enabled: Option<bool>,
    #[serde(rename = "client-id")]
    pub client_id: Option<String>,
    #[serde(rename = "client-secret")]
    pub client_secret: Option<String>,
    #[serde(rename = "issue-url")]
    pub issue_url: Option<String>,
    #[serde(rename = "redirect-url")]
    pub redirect_url: Option<String>,
    #[serde(rename = "scope")]
    pub scope: Option<String>,
}

// see:https://github.com/settings/developers
// see:https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuth2Properties {
    pub enabled: Option<bool>,
    #[serde(rename = "client-id")]
    pub client_id: Option<String>,
    #[serde(rename = "client-secret")]
    pub client_secret: Option<String>,
    #[serde(rename = "auth-url")]
    pub auth_url: Option<String>,
    #[serde(rename = "token-url")]
    pub token_url: Option<String>,
    #[serde(rename = "redirect-url")]
    pub redirect_url: Option<String>,
    // see:https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps
    #[serde(rename = "scope")]
    pub scope: Option<String>,
    #[serde(rename = "user-info-url")]
    pub user_info_url: Option<String>,
}

// see:https://github.com/settings/developers
// see:https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GithubProperties(OAuth2Properties);

/// Copy all OAuth2Config functions to GithubConfig.
impl Deref for GithubProperties {
    type Target = OAuth2Properties;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

//
// Cache Properties.
//
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheProperties {
    pub provider: CacheProvider,
    pub memory: MemoryProperties,
    pub redis: RedisProperties,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CacheProvider {
    MEMORY,
    REDIS,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryProperties {
    #[serde(rename = "initial-capacity")]
    pub initial_capacity: Option<u32>,
    #[serde(rename = "max-capacity")]
    pub max_capacity: Option<u64>,
    pub ttl: Option<u64>,
    #[serde(rename = "eviction-policy")]
    pub eviction_policy: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RedisProperties {
    pub nodes: Vec<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(rename = "connection-timeout")]
    pub connection_timeout: Option<u64>,
    #[serde(rename = "response-timeout")]
    pub response_timeout: Option<u64>,
    pub retries: Option<u32>,
    #[serde(rename = "max-retry-wait")]
    pub max_retry_wait: Option<u64>,
    #[serde(rename = "min-retry-wait")]
    pub min_retry_wait: Option<u64>,
    #[serde(rename = "read-from-replicas")]
    pub read_from_replicas: Option<bool>,
}

//
// App DB Properties.
//
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppDBProperties {
    #[serde(rename = "type")]
    pub db_type: AppDBType,
    #[serde(rename = "sqlite", default = "SqliteAppDBProperties::default")]
    pub sqlite: SqliteAppDBProperties,
    #[serde(rename = "postgres", default = "PostgresAppDBProperties::default")]
    pub postgres: PostgresAppDBProperties,
    #[serde(rename = "mongodb", default = "MongoAppDBProperties::default")]
    pub mongodb: MongoAppDBProperties,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum AppDBType {
    SQLITE,
    POSTGRESQL,
    MONGODB,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SqliteAppDBProperties {
    #[serde(rename = "dir")]
    pub dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostgresPropertiesBase {
    #[serde(rename = "host")]
    pub host: String,
    #[serde(rename = "port")]
    pub port: u16,
    #[serde(rename = "database")]
    pub database: String,
    #[serde(rename = "schema")]
    pub schema: String,
    #[serde(rename = "username")]
    pub username: String,
    #[serde(rename = "password")]
    pub password: Option<String>,
    #[serde(rename = "min-connections")]
    pub min_connections: Option<u32>,
    #[serde(rename = "max-connections")]
    pub max_connections: Option<u32>,
    #[serde(rename = "use-ssl")]
    pub use_ssl: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostgresAppDBProperties {
    #[serde(flatten)]
    pub inner: PostgresPropertiesBase,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MongoAppDBProperties {
    #[serde(rename = "url")]
    pub url: Option<String>,
    #[serde(rename = "database")]
    pub database: Option<String>,
}

//
// Vector DB Properties.
//
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VectorDBProperties {
    #[serde(rename = "type")]
    pub db_type: VectorDBType,
    #[serde(rename = "pg-vector", default = "PgVectorDBProperties::default")]
    pub pg_vector: PgVectorDBProperties,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum VectorDBType {
    PGVECTOR,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PgVectorDBProperties {
    #[serde(flatten)]
    pub inner: PostgresPropertiesBase,
}

//
// Services Properties.
//
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServicesProperties {
    #[serde(rename = "updaters")]
    pub updaters: Option<Vec<UpdaterProperties>>,
    #[serde(rename = "llm", default = "LlmProperties::default")]
    pub llm: LlmProperties,
}

/// Chain data Updater based LLM, Supports multi different L1 chains, as well as different environment networks of the same L1 chain.
#[derive(Debug, Serialize, Clone)]
pub struct UpdaterProperties {
    #[serde(rename = "kind")]
    pub kind: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "cron")]
    pub cron: String, // e.g: 0/30 * * * * * *
    #[serde(rename = "channel-size")]
    pub channel_size: usize, // e.g: 200
    #[serde(flatten)]
    pub chain: BlockChainProperties,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
//#[serde(tag = "chain-kind", content = "chain-config")]
#[serde(untagged)]
pub enum BlockChainProperties {
    ETHEREUM(EthereumChainProperties),
    SOLANA(SolanaChainProperties),
}

impl BlockChainProperties {
    pub fn is_ethereum(&self) -> bool {
        matches!(self, BlockChainProperties::ETHEREUM(_))
    }

    pub fn is_solana(&self) -> bool {
        matches!(self, BlockChainProperties::SOLANA(_))
    }

    pub fn as_ethereum(&self) -> &EthereumChainProperties {
        assert!(
            self.is_ethereum(),
            "Could not convert to Ethereum chain config from {:?}",
            self
        );
        if let BlockChainProperties::ETHEREUM(config) = self {
            config
        } else {
            panic!("Could not convert to Ethereum chain config from {:?}", self)
        }
    }

    pub fn as_solana(&self) -> &SolanaChainProperties {
        assert!(
            self.is_solana(),
            "Could not convert to Solana chain config from {:?}",
            self
        );
        if let BlockChainProperties::SOLANA(config) = self {
            config
        } else {
            panic!("Could not convert to Solana chain config from {:?}", self)
        }
    }
}

/// The custom deserialization to dynamic parse by kind.
impl<'de> Deserialize<'de> for UpdaterProperties {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut map = serde_json::Map::deserialize(deserializer)?;
        let chain_map = (map
            .to_owned()
            .remove("chain")
            .ok_or_else(|| serde::de::Error::custom("Missing 'chain' config field"))?)
        .as_object()
        .expect("Failed to parse the config field '.chain'")
        .to_owned();

        let kind = map
            .to_owned()
            .remove("kind")
            .ok_or_else(|| serde::de::Error::custom("Missing 'kind' field"))?
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("'kind' must be a string"))?
            .to_owned();

        let blockchain_config = match kind.to_uppercase().as_str() {
            "ETHEREUM" => BlockChainProperties::ETHEREUM(
                EthereumChainProperties::deserialize(Value::Object(chain_map.to_owned()))
                    .map_err(|e| serde::de::Error::custom(format!("Invalid Ethereum config: {}", e)))?,
            ),
            "SOLANA" => BlockChainProperties::SOLANA(
                SolanaChainProperties::deserialize(Value::Object(chain_map.to_owned()))
                    .map_err(|e| serde::de::Error::custom(format!("Invalid Solana config: {}", e)))?,
            ),
            _ => {
                return Err(serde::de::Error::custom(format!(
                    "Unsupported chain type in 'kind': {}",
                    kind
                )))
            }
        };

        Ok(Self {
            kind: kind.to_owned(),
            name: _pop_field(&mut map, "name").expect("Missing 'name' field"),
            enabled: _pop_field(&mut map, "enabled").expect("Missing 'enabled' field"),
            cron: _pop_field(&mut map, "cron").expect("Missing 'cron' field"),
            channel_size: _pop_field(&mut map, "channel-size").expect("Missing 'channel-size' field"),
            chain: blockchain_config,
        })
    }
}

/// The internal helper func to deserialize a get and remove the field.
fn _pop_field<'de, T: Deserialize<'de>>(
    map: &mut serde_json::Map<String, Value>,
    key: &str,
) -> Result<T, anyhow::Error> {
    let value = map
        .remove(key)
        .ok_or_else(|| anyhow::anyhow!(format!("Missing field '{}'", key)))?;
    T::deserialize(value).map_err(|e| anyhow::anyhow!(format!("Invalid value for '{}': {}", key, e)))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EthereumChainProperties {
    #[serde(rename = "chain-id")]
    pub chain_id: String, // e.g: 1
    #[serde(rename = "http-rpc-url")]
    pub http_rpc_url: String, // e.g: https://eth-mainnet.g.alchemy.com/v2/<YOUR_API_KEY>
    #[serde(rename = "ws-rpc-url")]
    pub ws_rpc_url: String, // e.g: wss://eth-mainnet.g.alchemy.com/v2/<YOUR_API_KEY>
    #[serde(rename = "contracts")]
    pub contracts: Option<Vec<EthereumContractProperties>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EthereumContractProperties {
    #[serde(rename = "address")]
    pub address: String, // e.g: ["0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419"]
    #[serde(rename = "abi-path")]
    pub abi_path: String, // e.g: '/etc/linkportal/abi/ethereum/uniswapv2factory-on-ethereum-mainnet-20250424.json'
    #[serde(rename = "event-names")]
    pub event_names: Vec<String>, // Target listen ethereum event names.
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SolanaChainProperties {
    #[serde(rename = "chain-id")]
    pub chain_id: String,
    #[serde(rename = "rpc-url")]
    pub rpc_url: String,
    #[serde(rename = "ws-url")]
    pub ws_url: Option<String>,
    #[serde(rename = "program-ids")]
    pub program_ids: Vec<String>,
    #[serde(rename = "account-filters")]
    pub account_filters: Option<Vec<String>>,
    #[serde(rename = "commitment")]
    pub commitment: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlmProperties {
    // see:https://platform.openai.com/docs/guides/completions
    // see:https://github.com/ollama/ollama/blob/main/docs/api.md#generate-a-completion
    // see:https://help.aliyun.com/zh/model-studio/getting-started/what-is-model-studio#16693d2e3fmir
    #[serde(rename = "embedding")]
    pub embedding: EmbeddingLLMProperties,
    #[serde(rename = "generate")]
    pub generate: GenerateLLMProperties,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmbeddingLLMProperties {
    #[serde(rename = "api-uri")]
    pub api_uri: String,
    #[serde(rename = "api-key")]
    pub api_key: Option<String>,
    #[serde(rename = "org-id")]
    pub org_id: Option<String>,
    #[serde(rename = "project-id")]
    pub project_id: Option<String>,
    #[serde(rename = "model")]
    pub model: String,
    #[serde(rename = "pre-delete-collection")]
    pub pre_delete_collection: bool,
    #[serde(rename = "vector-dimensions")]
    pub vector_dimensions: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerateLLMProperties {
    #[serde(rename = "api-uri")]
    pub api_uri: String,
    #[serde(rename = "api-key")]
    pub api_key: Option<String>,
    #[serde(rename = "org-id")]
    pub org_id: Option<String>,
    #[serde(rename = "project-id")]
    pub project_id: Option<String>,
    #[serde(rename = "model")]
    pub model: String,
    #[serde(rename = "max-tokens")]
    pub max_tokens: u32,
    #[serde(rename = "temperature")]
    pub temperature: f32,
    #[serde(rename = "candidate-count")]
    pub candidate_count: usize,
    #[serde(rename = "top-k")]
    pub top_k: usize,
    #[serde(rename = "top-p")]
    pub top_p: f32,
    #[serde(rename = "system-prompt")]
    pub system_prompt: String,
}

//
// App Properties impls.
//
impl AppConfigProperties {
    pub fn default() -> AppConfigProperties {
        AppConfigProperties {
            service_name: String::from("linkportal"),
            server: ServerProperties::default(),
            mgmt: MgmtProperties::default(),
            logging: LoggingProperties::default(),
            swagger: SwaggerProperties::default(),
            auth: AuthProperties::default(),
            cache: CacheProperties::default(),
            appdb: AppDBProperties::default(),
            vecdb: VectorDBProperties::default(),
            services: ServicesProperties::default(),
        }
    }
}

impl Default for ServerProperties {
    fn default() -> Self {
        ServerProperties {
            host: String::from("127.0.0.1"),
            port: 9000,
            context_path: None,
        }
    }
}

impl ServerProperties {
    pub fn get_bind_addr(&self) -> String {
        self.host.to_owned() + ":" + &self.port.to_string()
    }
}

//
// Management Properties impls.
//
impl Default for MgmtProperties {
    fn default() -> Self {
        MgmtProperties {
            enabled: true,
            host: String::from("127.0.0.1"),
            port: 9001,
            tokio_console: TokioConsoleProperties::default(),
            pyroscope: PyroscopeAgentProperties::default(),
            otel: OtelProperties::default(),
        }
    }
}

impl MgmtProperties {
    pub fn get_bind_addr(&self) -> String {
        self.host.to_owned() + ":" + &self.port.to_string()
    }
}

impl Default for TokioConsoleProperties {
    fn default() -> Self {
        TokioConsoleProperties {
            enabled: true,
            server_bind: String::from("0.0.0.0:6669"),
            retention: 60,
        }
    }
}

impl Default for PyroscopeAgentProperties {
    fn default() -> Self {
        PyroscopeAgentProperties {
            enabled: true,
            server_url: String::from("http://127.0.0.1:4040"),
            auth_token: None,
            tags: None,
            sample_rate: 0.1,
        }
    }
}

impl Default for OtelProperties {
    fn default() -> Self {
        OtelProperties {
            enabled: true,
            endpoint: String::from("http://localhost:4317"),
            protocol: String::from("grpc"),
            timeout: Some(Duration::from_secs(10).as_millis() as u64),
        }
    }
}

//
// Logging Properties impls.
//
impl Default for LoggingProperties {
    fn default() -> Self {
        LoggingProperties {
            mode: LogMode::JSON,
            level: "info".to_string(),
        }
    }
}

//
// Swagger Properties impls.
//
impl Default for SwaggerProperties {
    fn default() -> Self {
        SwaggerProperties {
            enabled: true,
            // title: "My Webnote API Server".to_string(),
            // description: "The My Webnote API Server".to_string(),
            // version: "1.0.0".to_string(),
            // license_name: "Apache 2.0".to_string(),
            // license_url: "https://www.apache.org/licenses/LICENSE-2.0".to_string(),
            // contact_name: "LinkPortal API".to_string(),
            // contact_email: "jameswong1376@gmail.com".to_string(),
            // contact_url: "https://github.com/wl4g/my-webnote".to_string(),
            // terms_of_service: "api/terms-of-service".to_string(),
            // //security_definitions: vec![],
            ui_path: "/swagger-ui".to_string(),
            openapi_url: "/api-docs/openapi.json".to_string(),
        }
    }
}

//
// Auth Properties impls.
//
impl Default for AuthProperties {
    fn default() -> Self {
        AuthProperties {
            jwt_ak_name: Some(String::from("_ak")),
            jwt_rk_name: Some(String::from("_rk")),
            jwt_validity_ak: Some(3600_000),
            jwt_validity_rk: Some(86400_000),
            jwt_secret: None,
            jwt_algorithm: None,
            anonymous_paths: None,
            oidc: OidcProperties::default(),
            github: GithubProperties::default(),
            login_url: Some(String::from("/static/login.html")),
            success_url: Some(String::from("/static/index.html")),
            unauthz_url: Some(String::from("/static/403.html")),
        }
    }
}

impl Default for OidcProperties {
    fn default() -> Self {
        OidcProperties {
            enabled: Some(false),
            client_id: None,
            client_secret: None,
            issue_url: None,
            redirect_url: None,
            scope: Some("openid profile email".to_string()),
        }
    }
}

impl Default for OAuth2Properties {
    fn default() -> Self {
        OAuth2Properties {
            enabled: Some(false),
            client_id: None,
            client_secret: None,
            auth_url: None,
            token_url: None,
            redirect_url: None,
            // see:https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps
            scope: Some("openid profile user:email user:follow read:user read:project public_repo".to_string()),
            user_info_url: None,
        }
    }
}

impl Default for GithubProperties {
    fn default() -> Self {
        // Beautifully impls for like java extends.
        GithubProperties(OAuth2Properties::default())
    }
}

//
// Cache Properties impls.
//
impl Default for CacheProperties {
    fn default() -> Self {
        CacheProperties {
            provider: CacheProvider::MEMORY,
            memory: MemoryProperties::default(),
            redis: RedisProperties::default(),
        }
    }
}

impl Default for MemoryProperties {
    fn default() -> Self {
        MemoryProperties {
            initial_capacity: Some(32),
            max_capacity: Some(65535),
            ttl: Some(3600),
            eviction_policy: Some("lru".to_string()),
        }
    }
}

impl Default for RedisProperties {
    fn default() -> Self {
        RedisProperties {
            nodes: vec!["redis://127.0.0.1:6379".to_string()],
            username: None,
            password: None,
            connection_timeout: Some(3000),
            response_timeout: Some(6000),
            retries: Some(1),
            max_retry_wait: Some(65536),
            min_retry_wait: Some(1280),
            read_from_replicas: Some(false),
        }
    }
}

//
// App DB Properties impls.
//
impl Default for AppDBProperties {
    fn default() -> Self {
        AppDBProperties {
            db_type: AppDBType::POSTGRESQL,
            sqlite: SqliteAppDBProperties::default(),
            postgres: PostgresAppDBProperties::default(),
            mongodb: MongoAppDBProperties::default(),
        }
    }
}

impl Default for SqliteAppDBProperties {
    fn default() -> Self {
        SqliteAppDBProperties {
            dir: Some(String::from("/tmp/linkportal/appdb/sqlite")),
        }
    }
}

impl Default for PostgresPropertiesBase {
    fn default() -> Self {
        PostgresPropertiesBase {
            host: String::from("127.0.0.1"),
            port: 5432,
            database: String::from("linkportal"),
            schema: String::from("linkportal"),
            username: String::from("postgres"),
            password: None,
            min_connections: Some(1),
            max_connections: Some(10),
            use_ssl: false,
        }
    }
}

impl Default for PostgresAppDBProperties {
    fn default() -> Self {
        PostgresAppDBProperties {
            inner: PostgresPropertiesBase::default(),
        }
    }
}

impl Deref for PostgresAppDBProperties {
    type Target = PostgresPropertiesBase;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Default for MongoAppDBProperties {
    fn default() -> Self {
        MongoAppDBProperties {
            url: Some(String::from("mongodb://localhost:27017")),
            database: Some(String::from("linkportal")),
        }
    }
}

//
// Vector DB Properties impls.
//
impl Default for VectorDBProperties {
    fn default() -> Self {
        VectorDBProperties {
            db_type: VectorDBType::PGVECTOR,
            pg_vector: PgVectorDBProperties::default(),
        }
    }
}

impl Default for PgVectorDBProperties {
    fn default() -> Self {
        PgVectorDBProperties {
            inner: PostgresPropertiesBase::default(),
        }
    }
}

impl Deref for PgVectorDBProperties {
    type Target = PostgresPropertiesBase;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

//
// Services Properties impls.
//
impl Default for ServicesProperties {
    fn default() -> Self {
        ServicesProperties {
            updaters: None,
            llm: LlmProperties::default(),
        }
    }
}

impl Default for LlmProperties {
    fn default() -> Self {
        LlmProperties {
            embedding: EmbeddingLLMProperties::default(),
            generate: GenerateLLMProperties::default(),
        }
    }
}

impl Default for EmbeddingLLMProperties {
    fn default() -> Self {
        EmbeddingLLMProperties {
            api_uri: String::from("https://dashscope.aliyuncs.com/compatible-mode/v1"),
            api_key: None,
            org_id: None,
            project_id: None,
            model: String::from("bge-m3:latest"),
            pre_delete_collection: false,
            vector_dimensions: 1536,
        }
    }
}

impl Default for GenerateLLMProperties {
    fn default() -> Self {
        GenerateLLMProperties {
            api_uri: String::from("https://dashscope.aliyuncs.com/compatible-mode/v1"),
            api_key: None,
            org_id: None,
            project_id: None,
            model: String::from("qwen-plus"),
            max_tokens: 65535,
            candidate_count: 1,
            temperature: 0.1,
            top_k: 1,
            top_p: 1.0,
            // TODO: default prompt or remove it.
            system_prompt: String::from("You are a blockchain expert. TODO TODO TODO ..."),
        }
    }
}

//
// App Configuration.
//
#[derive(Debug)]
pub struct AppConfig {
    pub inner: AppConfigProperties,
    pub auth_jwt_ak_name: String,
    pub auth_jwt_rk_name: String,
    pub auth_jwt_secret: String,
    pub auth_jwt_algorithm: Algorithm,
    pub auth_anonymous_glob_matcher: Option<GlobSet>,
}

impl Deref for AppConfig {
    type Target = AppConfigProperties;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AppConfig {
    pub fn new(config: &AppConfigProperties) -> Arc<AppConfig> {
        // Build to auth anonymous glob matcher.
        let globset;
        if config.auth.anonymous_paths.is_some() {
            let mut builder = GlobSetBuilder::new();
            for path in config.auth.anonymous_paths.as_ref().unwrap() {
                builder.add(Glob::new(path).unwrap());
            }
            globset = Some(builder.build().unwrap());
        } else {
            // Add built-in components routes to defaults.
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new(HEALTHZ_URI).unwrap());
            builder.add(Glob::new(format!("{}/**", HEALTHZ_URI).as_str()).unwrap());
            // The default accessing to swagger ui required authentication.
            //builder.add(Glob::new(&config.swagger.swagger_ui_path).unwrap());
            //builder.add(Glob::new(&config.swagger.swagger_openapi_url).unwrap());
            builder.add(Glob::new("/public/**").unwrap());
            builder.add(Glob::new("/static/**").unwrap());
            globset = Some(builder.build().unwrap());
        }

        let jwt_secret = match config.auth.jwt_secret.to_owned() {
            Some(secret) => secret,
            None => {
                let generated_jwt_secret = SecretHelper::generate_secret_base64(32).to_string();
                tracing::debug!("Generated the jwt secret: {}", generated_jwt_secret);
                generated_jwt_secret
            }
        };

        let auth_jwt_algorithm = Algorithm::from_str(
            config
                .auth
                .jwt_algorithm
                .to_owned()
                .unwrap_or(String::from("HS256"))
                .as_str(),
        )
        .ok()
        .expect("Invalid JWT algorithm configured");

        Arc::new(AppConfig {
            inner: config.clone(),
            auth_jwt_ak_name: config
                .auth
                .jwt_ak_name
                .to_owned()
                .unwrap_or(String::from("_ak"))
                .to_string(),
            auth_jwt_rk_name: config
                .auth
                .jwt_rk_name
                .to_owned()
                .unwrap_or(String::from("_rk"))
                .to_string(),
            auth_jwt_secret: jwt_secret,
            auth_jwt_algorithm,
            auth_anonymous_glob_matcher: globset,
        })
    }

    pub fn validate(self) -> Result<AppConfig, anyhow::Error> {
        // self.validate();
        Ok(self)
    }
}

fn init() -> Arc<AppConfig> {
    dotenv().ok(); // Notice: Must be called before parse from environment file (.env).

    let yaml_config = env::var("LINKPORTAL_CFG_PATH")
        .map(|path| {
            Config::builder()
                .add_source(config::File::with_name(path.as_str()))
                .add_source(
                    // Extrat candidate from env refer to: https://github.com/rust-cli/config-rs/blob/v0.15.9/src/env.rs#L290
                    // Set up into hierarchy struct attibutes refer to:https://github.com/rust-cli/config-rs/blob/v0.15.9/src/source.rs#L24
                    config::Environment::with_prefix("LINKPORTAL")
                        // Notice: Use double "_" to distinguish between different hierarchy struct or attribute alies at the same level.
                        .separator("__")
                        .convert_case(config::Case::Cobol)
                        .keep_prefix(false), // Remove the prefix when matching.
                )
                .build()
                .unwrap_or_else(|err| panic!("Error parsing config: {}", err))
                .try_deserialize::<AppConfigProperties>()
                .unwrap_or_else(|err| panic!("Error deserialize config: {}", err))
        })
        .unwrap_or(AppConfigProperties::default());

    let config = AppConfig::new(&yaml_config);

    if env::var("LINKPORTAL_CFG_VERBOSE").is_ok() || env::var("VERBOSE").is_ok() {
        println!("If you don't want to print the loaded configuration details, you can disable it by set up LINKPORTAL_CFG_VERBOSE=false.");
        println!(
            "Loaded the config details: {}",
            serde_json::to_string(&config.to_owned().inner).unwrap()
        );
    }

    return config;
}

pub fn get_config() -> Arc<AppConfig> {
    CONFIG.load().clone()
}

pub fn refresh_config() -> Result<(), anyhow::Error> {
    CONFIG.store(init());
    Ok(())
}

// Global the single refreshable configuration instance.
// see: https://github.com/wl4g-collect/openobserve/blob/v0.10.9/src/config/src/config.rs#L186
static CONFIG: Lazy<ArcSwap<AppConfig>> = Lazy::new(|| ArcSwap::from(init()));
