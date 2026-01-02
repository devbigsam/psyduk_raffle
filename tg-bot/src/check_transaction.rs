use  std::collections::HashSet;
use std::convert::TryInto;
use teloxide::prelude::*;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
    message::{VersionedMessage, Message},
    instruction::CompiledInstruction,
};
use solana_transaction_status::{UiTransactionEncoding, EncodedTransaction};
use bincode;
use std::str::FromStr;
use tokio::time::{Duration, sleep};

const MAX_ATTEMPTS: usize = 5; // Maximum retries for fetching transaction details

/// Monitors Solana transactions to detect payments and sends appropriate bot messages.
pub async fn monitor_transaction(
    user_wallet: String,
    raffle_wallet: &str,
    expected_amount: u64,
    bot: AutoSend<Bot>,
    chat_id: ChatId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use solana_client::rpc_config::RpcTransactionConfig;
    use solana_sdk::commitment_config::CommitmentConfig;

    let client = RpcClient::new_with_timeout(
        "https://boldest-holy-gadget.solana-devnet.quiknode.pro/43e556824f4d9fdb5fc2f614c2560d8eebf0fc97",
        Duration::from_secs(15),
    );

    let raffle_pubkey = Pubkey::from_str(raffle_wallet)?;
    let user_pubkey = Pubkey::from_str(&user_wallet)?;

    let mut seen_signatures: HashSet<String> = HashSet::new();
    if let Ok(signatures) = client.get_signatures_for_address(&raffle_pubkey) {
        seen_signatures.extend(signatures.into_iter().map(|s| s.signature));
    }

    for attempt in 0..30 {
        eprintln!("Debug: Attempt {}/30 to monitor transactions...", attempt + 1);

        match client.get_signatures_for_address(&raffle_pubkey) {
            Ok(signatures) => {
                for signature_info in signatures {
                    if seen_signatures.contains(&signature_info.signature) {
                        continue;
                    }

                    seen_signatures.insert(signature_info.signature.clone());
                    let signature = Signature::from_str(&signature_info.signature)?;

                    for fetch_attempt in 0..MAX_ATTEMPTS {
                        match client.get_transaction_with_config(
                            &signature,
                            RpcTransactionConfig {
                                encoding: Some(UiTransactionEncoding::Base64),
                                commitment: Some(CommitmentConfig::confirmed()),
                                max_supported_transaction_version: Some(0),
                            },
                        ) {
                            Ok(transaction_meta) => {
                                eprintln!(
                                    "Debug: Transaction fetched successfully on attempt {}: {:?}",
                                    fetch_attempt + 1, signature
                                );

                                match &transaction_meta.transaction.transaction {
                                    EncodedTransaction::LegacyBinary(encoded, )
                                    | EncodedTransaction::Binary(encoded, _) => {
                                        let tx: VersionedTransaction = bincode::deserialize(&base64::decode(encoded)?)?;

                                        if validate_transaction(&tx, &user_pubkey, &raffle_pubkey, expected_amount) {
                                            eprintln!("✅ Valid payment detected for {} lamports.", expected_amount);

                                            let _ = bot.send_message(
                                                chat_id,
                                                "✅ Payment received! You are now part of the current pool. Use /jackpot to check the pool size.",
                                            ).await;

                                            return Ok(());
                                        }
                                    }
                                    _ => {
                                        eprintln!("Unsupported transaction encoding");
                                    }
                                }
                                break;
                            }
                            Err(err) => {
                                eprintln!(
                                    "Debug: Error fetching transaction details on attempt {} for signature {}: {:?}",
                                    fetch_attempt + 1, signature, err
                                );

                                if fetch_attempt == MAX_ATTEMPTS - 1 {
                                    eprintln!(
                                        "Debug: Maximum retries reached for signature: {}. Skipping.",
                                        signature
                                    );
                                } else {
                                    sleep(Duration::from_secs(2u64.pow(fetch_attempt as u32))).await;
                                }
                            }
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("Debug: Error fetching signatures for raffle wallet: {:?}", err);
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        }

        sleep(Duration::from_secs(30)).await;
    }

    let _ = bot.send_message(chat_id,
        "❌ Payment not detected. Please ensure you sent the correct amount to the raffle wallet. If the issue persists, please try again or contact support.",
    ).await;

    Ok(())
}



/// Validates if a Solana transaction matches the expected details.
fn validate_transaction(
    transaction: &VersionedTransaction,
    user_pubkey: &Pubkey,
    raffle_pubkey: &Pubkey,
    expected_amount: u64,
) -> bool {
    eprintln!("Debug: Validating transaction: {:?}", transaction);
    match &transaction.message {
        VersionedMessage::Legacy(message) => {
            validate_legacy_message(message, user_pubkey, raffle_pubkey, expected_amount)
        }
        VersionedMessage::V0(message) => {
            validate_v0_message(message, transaction, user_pubkey, raffle_pubkey, expected_amount)
        }
    }
}

/// Validates legacy messages.
fn validate_legacy_message(
    message: &Message,
    user_pubkey: &Pubkey,
    raffle_pubkey: &Pubkey,
    expected_amount: u64,
) -> bool {
    let account_keys = &message.account_keys; // Extract account keys from the legacy message
    for instruction in &message.instructions {
        if is_valid_transfer(instruction, user_pubkey, raffle_pubkey, expected_amount, account_keys) {
            return true;
        }
    }
    false
}

/// Validates version 0 messages.
fn validate_v0_message(
    message: &solana_sdk::message::v0::Message,
    transaction: &VersionedTransaction,
    user_pubkey: &Pubkey,
    raffle_pubkey: &Pubkey,
    expected_amount: u64,
) -> bool {
    let account_keys = &transaction.message.static_account_keys();

    for instruction in &message.instructions {
        if is_valid_transfer(instruction, user_pubkey, raffle_pubkey, expected_amount, account_keys) {
            return true;
        }
    }
    false
}

/// Checks if an instruction is a valid transfer.
fn is_valid_transfer(
    instruction: &CompiledInstruction,
    user_pubkey: &Pubkey,
    raffle_pubkey: &Pubkey,
    expected_amount: u64,
    account_keys: &[Pubkey],
) -> bool {
    let system_program_id = solana_sdk::system_program::id();

    // Step 1: Check Program ID
    let program_id = account_keys.get(instruction.program_id_index as usize);
    eprintln!("Debug: Checking instruction with Program ID: {:?}", program_id);
    if program_id != Some(&system_program_id) {
        eprintln!(
            "Debug: Instruction skipped: Not a System Program instruction. Program ID: {:?}",
            program_id
        );
        return false;
    }

    // Step 2: Validate Accounts
    if instruction.accounts.len() < 2 {
        eprintln!(
            "Debug: Instruction skipped: Insufficient accounts. Accounts length: {}",
            instruction.accounts.len()
        );
        return false;
    }

    let default_pubkey = Pubkey::default();
    let source_pubkey = account_keys
        .get(instruction.accounts[0] as usize)
        .unwrap_or(&default_pubkey);
    let dest_pubkey = account_keys
        .get(instruction.accounts[1] as usize)
        .unwrap_or(&default_pubkey);

    eprintln!(
        "Debug: Instruction details: Source: {}, Destination: {}, Data Length: {}, Program ID: {}",
        source_pubkey,
        dest_pubkey,
        instruction.data.len(),
        program_id.unwrap_or(&default_pubkey)
    );

    if source_pubkey != user_pubkey || dest_pubkey != raffle_pubkey {
        eprintln!(
            "Debug: Source/Destination mismatch. Expected Source: {}, Destination: {}, but got Source: {}, Destination: {}",
            user_pubkey, raffle_pubkey, source_pubkey, dest_pubkey
        );
        return false;
    }

    // Step 3: Validate Transfer Amount
    if instruction.data.len() >= 12 {
        // Extract the transfer amount from bytes 4-12 (adjusted byte range)
        let transfer_amount = u64::from_le_bytes(
            instruction.data[4..12].try_into().expect("Failed to parse transfer amount"),
        );

        if transfer_amount == expected_amount {
            eprintln!(
                "✅ Debug: Valid transfer detected. Amount matches expected value: {} lamports.",
                transfer_amount
            );
            return true;
        } else {
            eprintln!(
                "Debug: Transfer amount mismatch. Expected: {} lamports, Found: {} lamports.",
                expected_amount, transfer_amount
            );
        }
    } else {
        eprintln!(
            "Debug: Instruction data size mismatch. Expected at least 12 bytes, Found: {} bytes.",
            instruction.data.len()
        );
    }

    false // If none of the checks pass
}