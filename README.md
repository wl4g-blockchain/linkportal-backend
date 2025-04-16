# LinkPortal for Backend

> A Open Source AI RWA Chain Portal Backend written in Rust.

## Introduction

LinkPortal Backend is a Rust-based high-performance service that empowers DApps by analyzing blockchain events and delivering AI-driven insights, enhancing capabilities while maintaining trust.

## Features

- **Blockchain Event Monitoring**: Real-time monitoring of on-chain transactions and events from smart contracts, including minting ERC1155 NFTs, auctions, and ERC20 token transfers.
- **Data Aggregation & Analysis**: Collects and analyzes transaction data to generate insights, storing processed results in SQLite/PostgreSQL/MongoDB for further use.
- **AI-Powered Risk Management**: Utilizes AI algorithms to predict potential loan liquidations in the StandalonePoolManager DeFi pool, optimizing gas usage by minimizing unnecessary liquidation function calls.
- **Real-Time Analytics Dashboard**: Provides near-real-time statistics such as annual percentage yield (APY), borrowing rates, and total RWA asset amounts for display in the DApp.
- **Customizable Data Visualization**: Generates interactive line charts and other visualizations to represent key metrics like APY trends and RWA asset values.
- **Integration with DeFi Protocols**: Supports seamless interaction with AAVE for staking, borrowing, and lending, while maintaining an independent DeFi pool with distinct risk/reward profiles.
- **Scalable Backend Architecture**: Built in Rust for high performance and reliability, ensuring efficient processing of large volumes of blockchain data.

## Development

- [Prerequisites for locally Development](./docs/devel/1.prerequisites-for-dev.md)
- [Build and Test for locally Development](./docs/devel/2.build-and-test-for-dev.md)

## Deployment

- [Deploy on Docker](./docs/deploy/deploy-on-docker.md)
- [Deploy on Kubernetes](./docs/deploy/deploy-on-kubernetes.md)
