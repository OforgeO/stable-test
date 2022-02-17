import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { StableTest } from '../target/types/stable_test';
import { TOKEN_PROGRAM_ID, Token, ASSOCIATED_TOKEN_PROGRAM_ID, } from '@solana/spl-token';
import { assert } from "chai";
import { PublicKey, SystemProgram, Transaction } from '@solana/web3.js';

describe('stable-test', () => {

  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.StableTest as Program<StableTest>;

  let escrow_account_pda = null;
  let escrow_account_bump = null;
  let stable_account_pda = null;
  let stable_account_bump = null;
  let sol_price_account_pda = null;
  let sol_price_account_bump = null;
  let stable_token = null;
  let token_authority = null;
  let token_authority_bump = null;

  const userAccount = anchor.web3.Keypair.generate();

  let pythAccount = new PublicKey("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix")

  it('Is initialized!', async () => {

    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(userAccount.publicKey, 2000000000),
      "confirmed"
    ); 

    [token_authority, token_authority_bump] = await PublicKey.findProgramAddress([
      Buffer.from("mint-authority"),
    ], program.programId);

    stable_token = await Token.createMint(
      provider.connection,
      userAccount,
      token_authority,
      null,
      9, // Decimal is 6
      TOKEN_PROGRAM_ID,
    );

    [escrow_account_pda, escrow_account_bump] = await PublicKey.findProgramAddress([
      Buffer.from("escrow"),
    ], program.programId);

    // [stable_account_pda, stable_account_bump] = await PublicKey.findProgramAddress([
    //   Buffer.from("stable"),
    // ], program.programId);

    [sol_price_account_pda, sol_price_account_bump] = await PublicKey.findProgramAddress([
      Buffer.from("sol_price"),
    ], program.programId);

    const escrow_amount = "1000000000";

    stable_account_pda = await stable_token.createAccount(userAccount.publicKey);

    // Add your test here.
    await program.rpc.processEscrow(
      escrow_account_bump,
      sol_price_account_bump,
      token_authority_bump,
      new anchor.BN(escrow_amount),
      {
        accounts: {
          userAccount: userAccount.publicKey,
          stableToken: stable_token.publicKey,
          tokenAuthority: token_authority,
          escrowAccount: escrow_account_pda,
          stableAccount: stable_account_pda,
          solPriceAccount: sol_price_account_pda,
          pythAccount: pythAccount,
          systemProgram: anchor.web3.SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [userAccount]
      }
    );

  });

  it('Mint Burn tokens', async () => {
    await program.rpc.processMintBurnToken(
      token_authority_bump,
      {
        accounts: {
          userAccount: userAccount.publicKey,
          stableToken: stable_token.publicKey,
          tokenAuthority: token_authority,
          stableAccount: stable_account_pda,
          solPriceAccount: sol_price_account_pda,
          pythAccount: pythAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [userAccount]
      }
    );
  });
});
