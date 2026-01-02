use teloxide::prelude::*;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_account_decoder::UiAccountData;
use solana_account_decoder::parse_account_data::ParsedAccount;
use std::str::FromStr;
use tokio::sync::RwLock;
use lazy_static::lazy_static;
use teloxide::types::InlineKeyboardButton;
use log::error;
use std::env;

const REQUIRED_TOKEN_MINT: &str = "iQuoGfqmXh6J3PShHDntayXGVixfp44wzGkVaH8r8RE"; // Token Mint
const MINIMUM_BALANCE: u64 = 0; // Minimum balance for eligibility

lazy_static! {
    static ref PENDING_USERS: RwLock<std::collections::HashMap<ChatId, String>> =
        RwLock::new(std::collections::HashMap::new());
}

/// Main eligibility check function
pub async fn check_user_eligibility(
    bot: AutoSend<Bot>,
    chat_id: ChatId,
    user_wallet: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Fetch token balance
    let token_balance = get_token_balance(user_wallet, REQUIRED_TOKEN_MINT).await;

    match token_balance {
        Some(balance) if balance >= MINIMUM_BALANCE => {
            // User is eligible
            bot.send_message(chat_id, "✅ You are eligible to participate!")
                .await?;

            // Ask how many tickets they want
            bot.send_message(chat_id, "How many tickets would you like to buy?")
                .await?;

            // Store the user's wallet address for later
            let mut pending_users = PENDING_USERS.write().await;
            pending_users.insert(chat_id, user_wallet.to_string());
        }
        Some(balance) => {
            // User does not hold enough tokens
            let buy_tokens_button = InlineKeyboardButton::url(
                "Buy Tokens",
                "https://example.com/buy_tokens".parse().unwrap(),
            );
            let keyboard = teloxide::types::InlineKeyboardMarkup::new(vec![vec![buy_tokens_button]]);

            bot.send_message(
                chat_id,
                format!(
                    "🚫 You need at least {} tokens to participate. Your balance: {}. Please acquire more tokens.",
                    MINIMUM_BALANCE, balance
                ),
            )
            .reply_markup(keyboard)
            .await?;
        }
        None => {
            // Error while fetching token balance
            bot.send_message(
                chat_id,
                "⚠️ Unable to check your token balance. Please verify your wallet address or try again later.",
            )
            .await?;
        }
    }

    Ok(())
}

/// Helper function to get token balance from Solana RPC
async fn get_token_balance(wallet: &str, mint_address: &str) -> Option<u64> {
    let rpc_url = env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let client = RpcClient::new(rpc_url);

    match client.get_token_accounts_by_owner(
        &Pubkey::from_str(wallet).ok()?,
        TokenAccountsFilter::Mint(Pubkey::from_str(mint_address).ok()?),
    ) {
        Ok(accounts) => {
            for account in accounts {
                if let Some(balance) = extract_token_balance(&account.account.data) {
                    return Some(balance);
                }
            }
            Some(0) // Return 0 if no accounts match the token mint
        }
        Err(err) => {
            error!("Error fetching token accounts: {}", err);
            None
        }
    }
}

/// Extracts token balance from account data
fn extract_token_balance(data: &UiAccountData) -> Option<u64> {
    match data {
        UiAccountData::Binary(encoded_data, _) => {
            if let Ok(decoded_data) = base64::decode(encoded_data) {
                if decoded_data.len() >= 72 {
                    return Some(u64::from_le_bytes(decoded_data[64..72].try_into().ok()?));
                }
            }
        }
        UiAccountData::Json(parsed_account) => {
            return extract_token_balance_from_parsed_account(parsed_account);
        }
        _ => {}
    }
    None
}

/// Extract token balance from ParsedAccount
fn extract_token_balance_from_parsed_account(parsed_account: &ParsedAccount) -> Option<u64> {
    parsed_account
        .parsed
        .get("info")
        .and_then(|info| info.as_object())
        .and_then(|info| info.get("tokenAmount"))
        .and_then(|token_amount| token_amount.as_object())
        .and_then(|token_amount| token_amount.get("amount"))
        .and_then(|amount_str| amount_str.as_str()?.parse::<u64>().ok())
}

