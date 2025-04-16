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

use super::server::WebServer;
use crate::cmd::management::ManagementServer;
use clap::Command;
use linkportal_server::config::config::AppConfig;
use linkportal_server::llm::handler::llm_base::LLMManager;
use linkportal_server::{
    config::config::{self, GIT_BUILD_DATE, GIT_COMMIT_HASH, GIT_VERSION},
    mgmt::apm,
};
use linkportal_updater::updater_base::LinkPortalUpdaterManager;
use linkportal_utils::panics::PanicHelper;
use std::env;
use std::sync::Arc;
use tokio::sync::oneshot;

pub struct StandaloneServer {}

impl StandaloneServer {
    pub const COMMAND_NAME: &'static str = "standalone";

    pub fn build() -> Command {
        Command::new(Self::COMMAND_NAME).about("Run LinkPortal Backend All Components in One with Standalone.")
    }

    #[allow(unused)]
    #[tokio::main]
    pub async fn run(matches: &clap::ArgMatches, verbose: bool) -> () {
        PanicHelper::set_hook_default();

        let config = config::get_config();

        Self::print_banner(config.to_owned(), verbose);

        // Initial APM components.
        apm::init_components(&config).await;

        let (signal_s, signal_r) = oneshot::channel();
        let signal_handle = ManagementServer::start(&config, true, signal_s).await;

        signal_r.await.expect("Failed to start Management server.");
        tracing::info!("Management server is ready on {}", config.mgmt.get_bind_addr());

        Self::start(&config, true).await;

        signal_handle.await.unwrap();
    }

    #[allow(unused)]
    async fn start(config: &Arc<AppConfig>, verbose: bool) {
        LLMManager::init().await;
        LinkPortalUpdaterManager::init().await;
        WebServer::start(config, verbose, None, None).await;
    }

    fn print_banner(config: Arc<AppConfig>, verbose: bool) {
        // http://www.network-science.de/ascii/#larry3d,graffiti,basic,drpepper,rounded,roman
        let ascii_name = r#"
 __                   __          ____               __             ___      
/\ \       __        /\ \        /\  _`\            /\ \__         /\_ \     
\ \ \     /\_\    ___\ \ \/'\    \ \ \L\ \___   _ __\ \ ,_\    __  \//\ \    
 \ \ \  __\/\ \ /' _ `\ \ , <     \ \ ,__/ __`\/\`'__\ \ \/  /'__`\  \ \ \   
  \ \ \L\ \\ \ \/\ \/\ \ \ \\`\    \ \ \/\ \L\ \ \ \/ \ \ \_/\ \L\.\_ \_\ \_ 
   \ \____/ \ \_\ \_\ \_\ \_\ \_\   \ \_\ \____/\ \_\  \ \__\ \__/.\_\/\____\
    \/___/   \/_/\/_/\/_/\/_/\/_/    \/_/\/___/  \/_/   \/__/\/__/\/_/\/____/  (Backend Standalone)
 "#;
        eprintln!("");
        eprintln!("{}", ascii_name);
        eprintln!("                Program Version: {:?}", GIT_VERSION);
        eprintln!(
            "                Package Version: {:?}",
            env!("CARGO_PKG_VERSION").to_string()
        );
        eprintln!("                Git Commit Hash: {:?}", GIT_COMMIT_HASH);
        eprintln!("                 Git Build Date: {:?}", GIT_BUILD_DATE);
        let path = env::var("LINKPORTAL_BACKEND_CFG_PATH").unwrap_or("none".to_string());
        eprintln!("        Configuration file path: {:?}", path);
        eprintln!(
            "            Web Serve listen on: \"{}://{}:{}\"",
            "http", &config.server.host, config.server.port
        );
        if config.mgmt.enabled {
            eprintln!(
                "     Management serve listen on: \"{}://{}:{}\"",
                "http", config.mgmt.host, config.mgmt.port
            );
            if config.mgmt.tokio_console.enabled {
                #[cfg(feature = "profiling-tokio-console")]
                let server_addr = &config.mgmt.tokio_console.server_bind;
                #[cfg(feature = "profiling-tokio-console")]
                eprintln!("   TokioConsole serve listen on: \"{}://{}\"", "http", server_addr);
            }
            if config.mgmt.pyroscope.enabled {
                #[cfg(feature = "profiling-pyroscope")]
                let server_url = &config.mgmt.pyroscope.server_url;
                #[cfg(feature = "profiling-pyroscope")]
                eprintln!("     Pyroscope agent connect to: \"{}\"", server_url);
            }
            if config.mgmt.otel.enabled {
                let endpoint = &config.mgmt.otel.endpoint;
                eprintln!("          Otel agent connect to: \"{}\"", endpoint);
            }
        }
        if verbose {
            let config_json = serde_json::to_string(&config.inner).unwrap_or_default();
            eprintln!("Configuration loaded: {}", config_json);
        }
        eprintln!("");
    }
}
