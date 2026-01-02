const {
    Connection,
    PublicKey,
    Keypair,
    Transaction,
    SystemProgram,
} = require("@solana/web3.js");
const TelegramBot = require("node-telegram-bot-api");

const RaffleProgramID = new PublicKey("87JSCiht1TyXmT1yHbYZpKGtgJRhKzBYyFrmENvAogef"); // Replace with your deployed program ID
const RaffleAccount = new PublicKey("3Qik6y2XjCymmam65y1s8Tm4MATUpaH18TKDa6TSvexv"); // Replace with your raffle account ID
const PayerKeypair = Keypair.fromSecretKey(Uint8Array.from([150,23,67,48,247,5,89,68,169,83,11,122,12,149,110,33,152,216,215,99,204,106,162,6,164,106,42,245,156,126,71,91,89,135,222,166,228,171,125,31,142,57,42,82,207,23,228,128,231,137,9,229,152,70,201,238,224,123,60,164,48,137,50,131])); // For sending transactions
const connection = new Connection("https://api.mainnet-beta.solana.com"); // Use mainnet or testnet endpoint

// Telegram bot setup
const TELEGRAM_API_TOKEN = "7652745861:AAEEU8H0aV9Ied9uwKd2wrzsMYFdvzvZzAc"; // Telegram Bot Token
const GROUP_CHAT_ID = "-4550881969"; // Replace with your group ID
const bot = new TelegramBot(TELEGRAM_API_TOKEN, { polling: false });

async function checkAndEndRaffle() {
    try {
        // Fetch the raffle account state
        const accountInfo = await connection.getAccountInfo(RaffleAccount);
        if (!accountInfo) throw new Error("Raffle account not found!");

        const data = accountInfo.data;
        const raffleState = parseRaffleState(data); // Custom function to parse your account state

        const currentTime = Math.floor(Date.now() / 1000);
        if (currentTime >= raffleState.end_time) {
            console.log("Raffle ended. Calling end_raffle...");

            // Create transaction to call `end_raffle`
            const instruction = createEndRaffleInstruction(RaffleProgramID, RaffleAccount, PayerKeypair.publicKey);
            const transaction = new Transaction().add(instruction);
            const signature = await connection.sendTransaction(transaction, [PayerKeypair]);
            console.log(`Transaction sent: ${signature}`);

            // Wait for confirmation
            await connection.confirmTransaction(signature, "confirmed");

            // Fetch the updated raffle state to get winner and jackpot details
            const updatedAccountInfo = await connection.getAccountInfo(RaffleAccount);
            const updatedRaffleState = parseRaffleState(updatedAccountInfo.data);

            // Send Telegram message with raffle results
            const winner = updatedRaffleState.winner || "No winner (No tickets sold)";
            const jackpot = updatedRaffleState.jackpot || 0;

            const message = `🎉 *Raffle Round Ended!*\n\n🏆 *Winner:* ${winner}\n💰 *Jackpot:* ${jackpot / 1e9} SOL\n\nThe next round starts now! Get your tickets! 🎟️`;

            await bot.sendMessage(GROUP_CHAT_ID, message, { parse_mode: "Markdown" });
        } else {
            console.log("Raffle still active. Next check...");
        }
    } catch (err) {
        console.error("Error in crank:", err);
    }
}

function createEndRaffleInstruction(programId, raffleAccount, payer) {
    const instructionData = Buffer.from([/* Custom data for end_raffle */]);
    return new SystemProgram({
        programId,
        keys: [
            { pubkey: raffleAccount, isSigner: false, isWritable: true },
            { pubkey: payer, isSigner: true, isWritable: false },
        ],
        data: instructionData,
    });
}


function parseRaffleState(data) {
    let offset = 0;

    // Parse jackpot (u64, 8 bytes)
    const jackpot = Number(data.readBigUInt64LE(offset));
    offset += 8;

    // Parse end_time (u64, 8 bytes)
    const end_time = Number(data.readBigUInt64LE(offset));
    offset += 8;

    // Parse winner (Pubkey, 32 bytes)
    const winner = new PublicKey(data.slice(offset, offset + 32)).toString();
    offset += 32;

    // Parse tickets (Vec<Pubkey>)
    const ticketCount = data.readUInt32LE(offset); // First 4 bytes indicate the vector length
    offset += 4;

    const tickets = [];
    for (let i = 0; i < ticketCount; i++) {
        const ticket = new PublicKey(data.slice(offset, offset + 32)).toString();
        tickets.push(ticket);
        offset += 32; // Move to the next Pubkey
    }

    return {
        jackpot,
        end_time,
        winner,
        tickets,
    };
}



// Run the crank script every minute
setInterval(checkAndEndRaffle, 60 * 1000);
