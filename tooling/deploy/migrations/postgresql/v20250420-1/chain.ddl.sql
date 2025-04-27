-- SPDX-License-Identifier: GNU GENERAL PUBLIC LICENSE Version 3
--
-- Copyleft (c) 2024 James Wong. This file is part of James Wong.
-- is free software: you can redistribute it and/or modify it under
-- the terms of the GNU General Public License as published by the
-- Free Software Foundation, either version 3 of the License, or
-- (at your option) any later version.
--
-- James Wong is distributed in the hope that it will be useful,
-- but WITHOUT ANY WARRANTY; without even the implied warranty of
-- MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
-- GNU General Public License for more details.
--
-- You should have received a copy of the GNU General Public License
-- along with James Wong.  If not, see <https://www.gnu.org/licenses/>.
--
-- IMPORTANT: Any software that fully or partially contains or uses materials
-- covered by this license must also be released under the GNU GPL license.
-- This includes modifications and derived works.
--
--
-- Create the ch_ethereum_event table
CREATE TABLE IF NOT EXISTS ch_ethereum_event (
    id BIGINT PRIMARY KEY,
    block_number BIGINT NOT NULL,
    transaction_hash VARCHAR(66) NOT NULL,
    contract_name VARCHAR(64) NOT NULL,
    contract_address VARCHAR(42) NOT NULL,
    event_name VARCHAR(255) NOT NULL,
    event_data JSONB NOT NULL,
    status INTEGER NULL default 0,
    create_by VARCHAR(64) NULL,
    create_time TIMESTAMPTZ default current_timestamp,
    update_by VARCHAR(64) NULL,
    update_time TIMESTAMPTZ default current_timestamp,
    del_flag INTEGER NOT NULL default 0,
    UNIQUE (transaction_hash, contract_address, event_name)
);
-- Create the All index for the ch_ethereum_event table.
CREATE INDEX IF NOT EXISTS idx_ch_ethereum_event_block_number ON ch_ethereum_event (block_number);
CREATE INDEX IF NOT EXISTS idx_ch_ethereum_event_contract_address ON ch_ethereum_event (contract_address);
CREATE INDEX IF NOT EXISTS idx_ch_ethereum_event_event_name ON ch_ethereum_event (event_name);
--
-- Create the ch_ethereum_checkpoint table
CREATE TABLE IF NOT EXISTS ch_ethereum_checkpoint (
    id BIGINT PRIMARY KEY,
    last_processed_block BIGINT NOT NULL,
    status INTEGER NULL default 0,
    create_by VARCHAR(64) NULL,
    create_time TIMESTAMPTZ default current_timestamp,
    -- create_time BIGINT DEFAULT (EXTRACT(EPOCH FROM current_timestamp) * 1000)::BIGINT,
    update_by VARCHAR(64) NULL,
    update_time TIMESTAMPTZ default current_timestamp,
    -- update_time BIGINT DEFAULT (EXTRACT(EPOCH FROM current_timestamp) * 1000)::BIGINT,
    del_flag INTEGER not NULL default 0
);