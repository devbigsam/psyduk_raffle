import { Connection, Keypair, Transaction, TransactionInstruction, SystemProgram, PublicKey, clusterApiUrl } from '@solana/web3.js';

const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');

// Replace these with your actual keys
const payer = Keypair.generate();  // The account that will pay for the creation
const raffleAccount = Keypair.generate();  // The raffle account to be created

const PROGRAM_ID = new PublicKey("87JSCiht1TyXmT1yHbYZpKGtgJRhKzBYyFrmENvAogef");

(async () => {
    // Create the transaction instruction for creating the raffle account
    const createAccountInstruction = SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: raffleAccount.publicKey,
        lamports: await connection.getMinimumBalanceForRentExemption(0), // Minimum lamports to avoid rent
        space: 0, // Size of the account to be created
        programId: PROGRAM_ID,  // The program ID for the raffle contract
    });

    // Create the transaction
    const transaction = new Transaction().add(createAccountInstruction);

    // Sign and send the transaction
    const signature = await connection.sendTransaction(transaction, [payer, raffleAccount], {
        preflightCommitment: 'processed',
    });

    console.log("Transaction signature:", signature);
})();
