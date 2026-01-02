# Psyduk Raffle Bot

## Description

Psyduk Raffle Bot is a Web3 utility that enables users to participate in raffles on Telegram using the Solana blockchain. Users can buy tickets with SOL (Solana's native cryptocurrency), and winners are selected randomly at the end of each raffle period. The project consists of a Solana program (built with Anchor) for handling raffle logic and a Rust-based Telegram bot for user interactions, eligibility checks, and transaction monitoring.

## Features

- **Telegram Bot Integration**: Interactive bot for users to start, check jackpot, buy tickets, and view winners.
- **Solana Program**: Smart contract for initializing raffles, buying tickets, and selecting winners using cryptographic randomness.
- **Eligibility Checks**: Verifies user eligibility based on holding a specific token (e.g., Psyduk token).
- **Transaction Monitoring**: Automatically detects and confirms SOL payments to the raffle wallet.
- **Automated Raffles**: Raffles run for 15 minutes, with winners selected and prizes distributed automatically.
- **Treasury System**: 20% of ticket sales go to a treasury wallet, 80% to the jackpot.

## Prerequisites

Before setting up the project, ensure you have the following installed:

- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)
- [Anchor Framework](https://www.anchor-lang.com/docs/installation)
- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/) (for Anchor and testing)
- A Telegram bot token (obtained from [@BotFather](https://t.me/botfather) on Telegram)
- Solana wallet with devnet SOL for testing

## Technologies and Tools

- **Rust**: Primary programming language for the Solana program and Telegram bot.
- **Anchor**: Framework for building secure Solana smart contracts.
- **Solana**: Blockchain platform for decentralized applications.
- **Node.js & TypeScript**: Used for testing, scripts, and Anchor integration.
- **Teloxide**: Rust crate for Telegram Bot API interactions.
- **Yarn**: Package manager for Node.js dependencies.
- **Prettier**: Code formatter for consistent styling.
- **Git**: Version control system for the repository.

## Installation

1. **Clone the Repository**:
   ```bash
   git clone https://github.com/devbigsam/psyduk_raffle.git
   cd psyduk_raffle
   ```

2. **Install Dependencies**:
   - For the Solana program:
     ```bash
     cd programs/psyduk_raffle
     anchor build
     ```
   - For the Telegram bot:
     ```bash
     cd tg-bot
     cargo build
     ```
   - For tests and scripts:
     ```bash
     npm install
     ```

3. **Set Up Environment Variables**:
   Create a `.env` file in the `tg-bot` directory with the following:
   ```
   TELOXIDE_TOKEN=your_telegram_bot_token_here
   SOLANA_RPC_URL=https://api.devnet.solana.com  # or mainnet-beta for production
   ```

4. **Configure Solana**:
   - Set Solana to devnet (or mainnet for production):
     ```bash
     solana config set --url https://api.devnet.solana.com
     ```
   - Airdrop SOL to your wallet if needed:
     ```bash
     solana airdrop 2
     ```

5. **Deploy the Solana Program**:
   ```bash
   anchor deploy
   ```
   Note the program ID and update it in the code if necessary.

6. **Build and Run the Telegram Bot**:
   ```bash
   cd tg-bot
   cargo run
   ```

## Usage

1. **Start the Bot**:
   Run the Telegram bot as described above. Users can interact with it via commands.

2. **Bot Commands**:
   - `/start`: Welcome message with rules and buttons.
   - `/jackpot`: View current jackpot and time to next draw.
   - `/buy`: Initiate ticket purchase process (checks eligibility, prompts for wallet and ticket count).
   - `/winners`: Display recent winners.

3. **Participating in Raffles**:
   - Users provide their Solana wallet address.
   - Bot checks eligibility (e.g., holding required tokens).
   - Users specify number of tickets (each 0.01 SOL).
   - Bot provides raffle wallet address for payment.
   - Transaction is monitored; upon confirmation, tickets are added to the raffle.
   - At raffle end, winner is selected and prize distributed.

4. **Testing**:
   - Use `anchor test` to run Solana program tests.
   - Interact with the bot on Telegram for end-to-end testing.

## Note on Completeness

This repository contains a partial implementation of the Psyduk Raffle Bot. It is not the complete version and may lack certain features, optimizations, or security enhancements. For the full, production-ready version suitable for your projects, please reach out to the developer on Telegram.

## Contact

For inquiries, support, or to obtain the complete version, contact the developer:

- **Telegram**: [@devbigsam](https://t.me/devbigsam)

## License

This project is licensed under the ISC License - see the [LICENSE](LICENSE) file for details.

---

*Disclaimer: This project is for educational and demonstration purposes. Use at your own risk. Ensure compliance with relevant laws and regulations when deploying Web3 applications.*

Developed by Samuel (BIG SAM)
# psyduk_raffle
