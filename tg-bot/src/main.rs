use teloxide::prelude::*;
use teloxide::types::{BotCommand, InlineKeyboardButton, InlineKeyboardMarkup, InputFile, CallbackQuery, ChatId};
use std::collections::HashMap;
use std::sync::Arc;
use dotenv::dotenv;
use tokio::sync::Mutex; // Use tokio's async Mutex
use lazy_static::lazy_static;
use std::env;

mod check_eligibility;
mod check_transaction;

// Shared state to track users who need to provide their wallet address or ticket count
lazy_static! {
static ref PENDING_WALLET_ADDRESSES: Arc<Mutex<HashMap<ChatId, (String, bool)>>>
    = Arc::new(Mutex::new(HashMap::new())); // Store wallet and eligibility status
static ref PENDING_TICKET_COUNT: Arc<Mutex<HashMap<ChatId, String>>> = Arc::new(Mutex::new(HashMap::new()));
}

// Constants
const RAFFLE_WALLET: &str = "3Qik6y2XjCymmam65y1s8Tm4MATUpaH18TKDa6TSvexv"; // Replace with actual raffle wallet address
const TICKET_PRICE: u64 = 10_000_000; // 0.01 SOL in lamports

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();
    pretty_env_logger::init();

    let bot_token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN environment variable not set");
    let bot = Bot::new(bot_token).auto_send();

    let bot_user = bot.get_me().await?;
    let bot_username = Arc::new(format!("@{}", bot_user.username.as_ref().unwrap())); // Use Arc for bot_username

    bot.set_my_commands(vec![
        BotCommand::new("/start", "Start Psyduk raffle bot"),
        BotCommand::new("/jackpot", "Show the current jackpot 🎰"),
        BotCommand::new("/winners", "Displays the last winners 🥇"),
        BotCommand::new("/buy", "Buy Tickets 🎟"),
    ])
    .await?;

    let handler = dptree::entry()
    .branch(Update::filter_callback_query().endpoint(handle_callback_query)) // Handles callback queries
    .branch(Update::filter_message().endpoint({
        let bot = bot.clone();
        let bot_username = Arc::clone(&bot_username);
        move |msg| handle_message(bot.clone(), msg, bot_username.clone())
    })); // Handles messages


    Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![bot.clone()])
        .default_handler(|_| async {})
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn handle_message(
    bot: AutoSend<Bot>, 
    message: Message, 
    bot_username: Arc<String>  // Use Arc<String> here
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(text) = message.text() {
        let is_private_chat = message.chat.is_private();
        let is_command_for_bot = text.starts_with('/') && (is_private_chat || text.contains(&*bot_username));

        if is_command_for_bot {
            let command = text.split('@').next().unwrap_or(text).trim();

            match command {
                "/start" => {
                    let buttons = InlineKeyboardMarkup::new(vec![
                        vec![
                            InlineKeyboardButton::callback("SEE JACKPOT 🎰", "jackpot"),
                            InlineKeyboardButton::callback("BUY TICKETS 🎟", "buy"),
                        ],
                    ]);                    

                    let image_url = "https://ibb.co/cYBDkKv"; // Update to actual URL

                    bot.send_photo(
                        message.chat.id,
                        InputFile::url(image_url.parse().unwrap())
                    )
                    .caption("🎉 Welcome to the PSYDUK Raffle Bot! 🎉 \n \n ➡ Rules: \n \n 1. Each ticket costs 0.01 SOL 🎟 \n2. Buy as many tickets as you want with the filter /buy. \n3. A draw is made every 15mins ⏳ \n4. The winner takes home the accumulated jackpot! /jackpot \n \n ➡ Check our last winners with the filter: /winners 🏆 \n Good Luck! 🍀")
                    .reply_markup(buttons)
                    .await?;
                }
                "/jackpot" => {
                    bot.send_message(
                        message.chat.id,
                        "🎰 Current Jackpot: 100 SOL 🎰\n\nThe next draw is in 10 minutes! ⏳\n\nTo participate, buy your tickets using /buy. Each ticket costs 0.01 SOL. Best of luck!"
                    ).await?;
                }
"/buy" => {
                    bot.send_message(
                        message.chat.id,
                        "Please send me your Solana wallet address to verify eligibility."
                    ).await?;

                    // Mark the chat as pending for wallet response
                    let mut pending_wallet_addresses = PENDING_WALLET_ADDRESSES.lock().await;
                    pending_wallet_addresses.insert(message.chat.id, (String::new(), false)); // false indicates pending eligibility
                }
                "/winners" => {
                    bot.send_message(
                        message.chat.id,
                        "🏆 Here are the recent winners:\n\n1. Wallet: ABCD...1234 - Won 50 SOL\n2. Wallet: EFGH...5678 - Won 30 SOL\n\nKeep participating to have a chance to win big! 🎉"
                    ).await?;
                }
                _ => {
                    if is_private_chat {
                        bot.send_message(
                            message.chat.id,
                            "To buy tickets, please follow these steps: 🎟️\n\n➡️ Send SOL to the following Solana address:\n\nGaCKAKASHIPsyDukCcaf8765300\n\n(For each 0.01 SOL you will get one ticket)\n\n➡️ Our automated system will detect your payment and send you a confirmation 💥"
                        ).await?;
                    }
                }
            }
        }
        // Respond to non-command messages if the user is expected to provide a wallet address
        else if is_private_chat {
            let mut pending_wallet_addresses = PENDING_WALLET_ADDRESSES.lock().await;

            if let Some(_) = pending_wallet_addresses.get(&message.chat.id) {
                // User sent their wallet address
                let user_wallet = text.trim().to_string();
                pending_wallet_addresses.remove(&message.chat.id); // Remove from pending list
                
                // Check eligibility
                check_eligibility::check_user_eligibility(bot.clone(), message.chat.id, &user_wallet).await?;

                // Mark the user as awaiting ticket count
                let mut pending_ticket_count = PENDING_TICKET_COUNT.lock().await;
                pending_ticket_count.insert(message.chat.id, user_wallet);
            } else if let Some(user_wallet) = PENDING_TICKET_COUNT.lock().await.get(&message.chat.id) {
                // User is providing ticket count
                let ticket_count: u64 = match text.trim().parse() {
                    Ok(count) => count,
                    Err(_) => {
                        bot.send_message(
                            message.chat.id,
                            "Please enter a valid number of tickets."
                        ).await?;
                        return Ok(());
                    }
                };

                let total_cost = ticket_count * TICKET_PRICE;

                bot.send_message(
                    message.chat.id,
                    format!(
                        "💳 Please send {} SOL to the following wallet address:\n\n{}\n\nWaiting for payment confirmation...",
                        total_cost as f64 / 1_000_000_000.0,
                        RAFFLE_WALLET
                    ),
                )
                .await?;

                // Monitor the transaction
                tokio::spawn({
                    let bot_clone = bot.clone();
                    let chat_id_clone = message.chat.id;
                    let user_wallet_clone = user_wallet.clone();
                    async move {
                        if let Err(e) = check_transaction::monitor_transaction(
                            user_wallet_clone,
                            RAFFLE_WALLET,
                            total_cost,
                            bot_clone,
                            chat_id_clone,
                        )
                        .await
                        {
                            eprintln!("Error in transaction monitoring for chat {}: {:?}", chat_id_clone, e);
                        }
                
                        // Cleanup state even if there is an error
                        let mut pending_tickets = PENDING_TICKET_COUNT.lock().await;
                        pending_tickets.remove(&chat_id_clone);
                    }
                });
                
            } else {
                bot.send_message(
                    message.chat.id,
                    "To buy tickets, please send /buy to confirm your wallet eligibility, if eligible you will be sent how to buy instruction."
                ).await?;
            }
        }
    }
    Ok(())
}

async fn handle_callback_query(
    bot: AutoSend<Bot>, 
    query: CallbackQuery,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(data) = query.data.clone() {
        match data.as_str() {
            "jackpot" => {
                bot.send_message(
                    query.from.id,
                    "🎰 Current Jackpot: 100 SOL 🎰\n\nThe next draw is in 10 minutes! ⏳"
                )
                .await?;
            }
            "buy" => {
                bot.send_message(
                    query.from.id,
                    "To buy tickets, please use the /buy command. Each ticket costs 0.01 SOL."
                )
                .await?;
            }
            _ => {
                bot.send_message(query.from.id, "Unknown action. Please try again.")
                    .await?;
            }
        }

        // Acknowledge the callback query
        bot.answer_callback_query(query.id).await?;
    } else {
        bot.send_message(query.from.id, "No callback data received.")
            .await?;
    }
    Ok(())
}
