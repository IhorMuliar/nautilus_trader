// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

use std::{collections::HashMap, sync::Arc};

use alloy::{primitives::Address, sol, sol_types::SolCall};

use super::base::{BaseContract, ContractCall, Multicall3};
use crate::rpc::{error::BlockchainRpcClientError, http::BlockchainHttpRpcClient};

sol! {
    #[sol(rpc)]
    contract ERC20 {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
    }
}

/// Represents the essential metadata information for an ERC20 token.
#[derive(Debug, Clone)]
pub struct TokenInfo {
    /// The full name of the token.
    pub name: String,
    /// The ticker symbol of the token.
    pub symbol: String,
    /// The number of decimal places the token uses for representing fractional amounts.
    pub decimals: u8,
}

/// Interface for interacting with ERC20 token contracts on a blockchain.
///
/// This struct provides methods to fetch token metadata (name, symbol, decimals).
/// From ERC20-compliant tokens on any EVM-compatible blockchain.
#[derive(Debug)]
pub struct Erc20Contract {
    /// The base contract providing common RPC execution functionality.
    base: BaseContract,
}

impl Erc20Contract {
    /// Creates a new ERC20 contract interface with the specified RPC client.
    #[must_use]
    pub const fn new(client: Arc<BlockchainHttpRpcClient>) -> Self {
        Self {
            base: BaseContract::new(client),
        }
    }

    /// Fetches complete token information (name, symbol, decimals) from an ERC20 contract.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the contract calls fail.
    /// - [`BlockchainRpcClientError::ClientError`] if an RPC call fails.
    /// - [`BlockchainRpcClientError::AbiDecodingError`] if ABI decoding fails.
    pub async fn fetch_token_info(
        &self,
        token_address: &Address,
    ) -> Result<TokenInfo, BlockchainRpcClientError> {
        // Try multicall first, fallback to individual calls if it fails
        match self.fetch_token_info_multicall(token_address).await {
            Ok(info) => Ok(info),
            Err(_) => {
                // Fallback to individual calls
                let token_name = self.fetch_name(token_address).await?;
                let token_symbol = self.fetch_symbol(token_address).await?;
                let token_decimals = self.fetch_decimals(token_address).await?;

                Ok(TokenInfo {
                    name: token_name,
                    symbol: token_symbol,
                    decimals: token_decimals,
                })
            }
        }
    }

    /// Fetches complete token information using multicall for efficiency.
    ///
    /// # Errors
    ///
    /// Returns an error if the multicall fails or decoding fails.
    pub async fn fetch_token_info_multicall(
        &self,
        token_address: &Address,
    ) -> Result<TokenInfo, BlockchainRpcClientError> {
        // Prepare the three calls
        let calls = vec![
            ContractCall {
                target: *token_address,
                allow_failure: false,
                call_data: ERC20::nameCall.abi_encode(),
            },
            ContractCall {
                target: *token_address,
                allow_failure: false,
                call_data: ERC20::symbolCall.abi_encode(),
            },
            ContractCall {
                target: *token_address,
                allow_failure: false,
                call_data: ERC20::decimalsCall.abi_encode(),
            },
        ];

        // Execute the multicall
        let results = self.base.execute_multicall(calls).await?;

        // Parse individual results
        if results.len() != 3 {
            return Err(BlockchainRpcClientError::AbiDecodingError(
                "Unexpected number of results from multicall".to_string(),
            ));
        }

        let name = parse_multicall_string_result(&results[0], "name")?;
        let symbol = parse_multicall_string_result(&results[1], "symbol")?;
        let decimals = parse_multicall_u8_result(&results[2], "decimals")?;

        Ok(TokenInfo {
            name,
            symbol,
            decimals,
        })
    }

    /// Fetches the name of an ERC20 token.
    async fn fetch_name(
        &self,
        token_address: &Address,
    ) -> Result<String, BlockchainRpcClientError> {
        let name_call = ERC20::nameCall.abi_encode();
        let bytes = self.base.execute_call(token_address, &name_call).await?;

        if bytes.is_empty() {
            return Err(BlockchainRpcClientError::AbiDecodingError(
                "Token does not implement name() function or returned empty response".to_string(),
            ));
        }

        ERC20::nameCall::abi_decode_returns(&bytes).map_err(|e| {
            BlockchainRpcClientError::AbiDecodingError(format!(
                "Error decoding ERC20 contract name with error {e}"
            ))
        })
    }

    /// Fetches the symbol of an ERC20 token.
    async fn fetch_symbol(
        &self,
        token_address: &Address,
    ) -> Result<String, BlockchainRpcClientError> {
        let symbol_call = ERC20::symbolCall.abi_encode();
        let bytes = self.base.execute_call(token_address, &symbol_call).await?;

        if bytes.is_empty() {
            return Err(BlockchainRpcClientError::AbiDecodingError(
                "Token not implement symbol() function or returned empty response".to_string(),
            ));
        }

        ERC20::symbolCall::abi_decode_returns(&bytes).map_err(|e| {
            BlockchainRpcClientError::AbiDecodingError(format!(
                "Error decoding ERC20 contract symbol with error {e}"
            ))
        })
    }

    /// Fetches the number of decimals used by an ERC20 token.
    async fn fetch_decimals(
        &self,
        token_address: &Address,
    ) -> Result<u8, BlockchainRpcClientError> {
        let decimals_call = ERC20::decimalsCall.abi_encode();
        let bytes = self.base.execute_call(token_address, &decimals_call).await?;

        if bytes.is_empty() {
            return Err(BlockchainRpcClientError::AbiDecodingError(
                "Token does not implement decimals() function or returned empty response"
                    .to_string(),
            ));
        }

        ERC20::decimalsCall::abi_decode_returns(&bytes).map_err(|e| {
            BlockchainRpcClientError::AbiDecodingError(format!(
                "Error decoding ERC20 contract decimals with error {e}"
            ))
        })
    }

    /// Fetches token information for multiple tokens in a single multicall.
    ///
    /// This method is highly efficient for fetching metadata for many tokens at once,
    /// reducing the number of RPC calls from N to 1.
    ///
    /// # Errors
    ///
    /// Returns an error if the multicall itself fails. Individual token failures
    /// are captured in the Result values of the returned HashMap.
    pub async fn batch_fetch_token_info(
        &self,
        token_addresses: &[Address],
    ) -> Result<HashMap<Address, Result<TokenInfo, String>>, BlockchainRpcClientError> {
        if token_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        // Build calls for all tokens (3 calls per token)
        let mut calls = Vec::with_capacity(token_addresses.len() * 3);
        
        for token_address in token_addresses {
            calls.extend([
                ContractCall {
                    target: *token_address,
                    allow_failure: true,  // Allow individual token failures
                    call_data: ERC20::nameCall.abi_encode(),
                },
                ContractCall {
                    target: *token_address,
                    allow_failure: true,
                    call_data: ERC20::symbolCall.abi_encode(),
                },
                ContractCall {
                    target: *token_address,
                    allow_failure: true,
                    call_data: ERC20::decimalsCall.abi_encode(),
                },
            ]);
        }

        // Execute all calls in one RPC
        let results = self.base.execute_multicall(calls).await?;
        
        // Process results in chunks of 3 (name, symbol, decimals)
        let mut token_infos = HashMap::with_capacity(token_addresses.len());
        
        for (i, token_address) in token_addresses.iter().enumerate() {
            let base_idx = i * 3;
            
            // Ensure we have all 3 results for this token
            if base_idx + 2 >= results.len() {
                token_infos.insert(
                    *token_address,
                    Err("Incomplete results from multicall".to_string()),
                );
                continue;
            }
            
            let token_info = parse_batch_token_results(&results[base_idx..base_idx + 3]);
            token_infos.insert(*token_address, token_info);
        }
        
        Ok(token_infos)
    }

    /// Fetches token information for multiple tokens with automatic chunking.
    ///
    /// This method handles large token lists by breaking them into smaller chunks
    /// to avoid hitting multicall size limits.
    ///
    /// # Arguments
    ///
    /// * `token_addresses` - The list of token addresses to fetch info for
    /// * `chunk_size` - Maximum number of tokens per multicall (recommended: 100-300)
    ///
    /// # Errors
    ///
    /// Returns an error if any multicall fails. Individual token failures
    /// are captured in the Result values of the returned HashMap.
    pub async fn batch_fetch_token_info_chunked(
        &self,
        token_addresses: &[Address],
        chunk_size: usize,
    ) -> Result<HashMap<Address, Result<TokenInfo, String>>, BlockchainRpcClientError> {
        let mut all_results = HashMap::with_capacity(token_addresses.len());
        
        for chunk in token_addresses.chunks(chunk_size) {
            let chunk_results = self.batch_fetch_token_info(chunk).await?;
            all_results.extend(chunk_results);
        }
        
        Ok(all_results)
    }
}

/// Parses a string result from a multicall response.
///
/// # Errors
///
/// Returns an error if the call failed or decoding fails.
fn parse_multicall_string_result(
    result: &Multicall3::Result,
    field_name: &str,
) -> Result<String, BlockchainRpcClientError> {
    if !result.success {
        return Err(BlockchainRpcClientError::AbiDecodingError(format!(
            "Multicall failed for {field_name}"
        )));
    }

    if result.returnData.is_empty() {
        return Err(BlockchainRpcClientError::AbiDecodingError(format!(
            "Empty response for {field_name}"
        )));
    }

    match field_name {
        "name" => ERC20::nameCall::abi_decode_returns(&result.returnData).map_err(|e| {
            BlockchainRpcClientError::AbiDecodingError(format!(
                "Failed to decode {field_name}: {e}"
            ))
        }),
        "symbol" => ERC20::symbolCall::abi_decode_returns(&result.returnData).map_err(|e| {
            BlockchainRpcClientError::AbiDecodingError(format!(
                "Failed to decode {field_name}: {e}"
            ))
        }),
        _ => Err(BlockchainRpcClientError::AbiDecodingError(format!(
            "Unknown field: {field_name}"
        ))),
    }
}

/// Parses a u8 result from a multicall response.
///
/// # Errors
///
/// Returns an error if the call failed or decoding fails.
fn parse_multicall_u8_result(
    result: &Multicall3::Result,
    field_name: &str,
) -> Result<u8, BlockchainRpcClientError> {
    if !result.success {
        return Err(BlockchainRpcClientError::AbiDecodingError(format!(
            "Multicall failed for {field_name}"
        )));
    }

    if result.returnData.is_empty() {
        return Err(BlockchainRpcClientError::AbiDecodingError(format!(
            "Empty response for {field_name}"
        )));
    }

    ERC20::decimalsCall::abi_decode_returns(&result.returnData).map_err(|e| {
        BlockchainRpcClientError::AbiDecodingError(format!(
            "Failed to decode {field_name}: {e}"
        ))
    })
}

/// Parses token information from a slice of 3 multicall results.
///
/// Expects results in order: name, symbol, decimals.
/// Returns Ok(TokenInfo) if all three calls succeeded, or an Err with a
/// descriptive error message if any call failed.
fn parse_batch_token_results(results: &[Multicall3::Result]) -> Result<TokenInfo, String> {
    if results.len() != 3 {
        return Err("Expected exactly 3 results per token".to_string());
    }

    // Parse name
    let name = if results[0].success && !results[0].returnData.is_empty() {
        ERC20::nameCall::abi_decode_returns(&results[0].returnData)
            .map_err(|e| format!("Failed to decode name: {e}"))?
    } else {
        return Err("Failed to fetch token name".to_string());
    };

    // Parse symbol
    let symbol = if results[1].success && !results[1].returnData.is_empty() {
        ERC20::symbolCall::abi_decode_returns(&results[1].returnData)
            .map_err(|e| format!("Failed to decode symbol: {e}"))?
    } else {
        return Err("Failed to fetch token symbol".to_string());
    };

    // Parse decimals
    let decimals = if results[2].success && !results[2].returnData.is_empty() {
        ERC20::decimalsCall::abi_decode_returns(&results[2].returnData)
            .map_err(|e| format!("Failed to decode decimals: {e}"))?
    } else {
        return Err("Failed to fetch token decimals".to_string());
    };

    Ok(TokenInfo {
        name,
        symbol,
        decimals,
    })
}
