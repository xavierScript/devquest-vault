// Import necessary libraries and types for Anchor, Solana, and testing
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DevquestVault } from "../target/types/devquest_vault";
import { Keypair } from "@solana/web3.js";
import { assert } from "chai";

// Main test suite for the devquest-vault program
describe("devquest-vault", () => {
  // Configure the client to use the local Solana cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Reference to the deployed program
  const program = anchor.workspace.DevquestVault as Program<DevquestVault>;

  // Generate test payee accounts and an unauthorized user
  const payee1 = Keypair.generate();
  const payee2 = Keypair.generate();
  const unauthorizedUser = Keypair.generate();

  // The admin is the provider's wallet
  const admin = provider.wallet.publicKey;

  // Derive the PDA (Program Derived Address) for the vault state and vault accounts
  const vaultState = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("state"), admin.toBytes()],
    program.programId
  )[0];
  const vault = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), vaultState.toBytes()],
    program.programId
  )[0];

  // Fund the test accounts with SOL for transaction fees
  before(async () => {
    // Airdrop SOL to payee1 for transaction fees
    const fundTx = await provider.connection.requestAirdrop(
      payee1.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(fundTx);
  });

  // Test: Initialize the vault and vault state
  it("Is initialized!", async () => {
    // Call the initialize method on the program
    const tx = await program.methods
      .initialize()
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
        vault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\nYour transaction signature", tx);
    console.log(
      "Your vault info",
      await provider.connection.getAccountInfo(vault)
    );
  });

  // Test: Deposit 2 SOL into the vault
  it("Deposit 2 SOL", async () => {
    const tx = await program.methods
      .deposit(new anchor.BN(2 * anchor.web3.LAMPORTS_PER_SOL))
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
        vault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\nYour transaction signature", tx);
    console.log(
      "Your vault info",
      await provider.connection.getAccountInfo(vault)
    );
    console.log(
      "Your vault balance",
      (await provider.connection.getBalance(vault)).toString()
    );
  });

  // Test: Withdraw 1 SOL from the vault as the admin
  it("Withdraw 1 SOL", async () => {
    const tx = await program.methods
      .withdraw(new anchor.BN(1 * anchor.web3.LAMPORTS_PER_SOL))
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
        vault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\nYour transaction signature", tx);
    console.log(
      "Your vault balance",
      (await provider.connection.getBalance(vault)).toString()
    );
  });

  // Test: Admin can add payees to the vault
  it("Admin can add payees", async () => {
    // Add first payee
    const tx1 = await program.methods
      .addPayee(payee1.publicKey)
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
      })
      .rpc();

    console.log("\nAdding first payee transaction signature", tx1);

    // Add second payee
    const tx2 = await program.methods
      .addPayee(payee2.publicKey)
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
      })
      .rpc();

    console.log("Adding second payee transaction signature", tx2);
  });

  // Test: Only admin can add payees (unauthorized users are prevented)
  it("Only admin can add payees", async () => {
    try {
      // Try to add a payee using unauthorized user (should fail)
      await program.methods
        .addPayee(unauthorizedUser.publicKey)
        .accountsPartial({
          user: payee1.publicKey,
          vaultState,
        })
        .signers([payee1])
        .rpc();

      assert.fail("Expected error when non-admin tries to add payee");
    } catch (error) {
      console.log("Successfully prevented unauthorized payee addition");
      assert.equal(error.error.errorCode.code, "UnauthorizedAdmin");
    }
  });

  // Test: Authorized payee can withdraw from the vault
  it("Authorized payee can withdraw", async () => {
    // First deposit some SOL to ensure vault has funds
    await program.methods
      .deposit(new anchor.BN(2 * anchor.web3.LAMPORTS_PER_SOL))
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
        vault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    // Payee1 tries to withdraw 0.5 SOL
    const withdrawAmount = new anchor.BN(0.5 * anchor.web3.LAMPORTS_PER_SOL);
    const tx = await program.methods
      .withdraw(withdrawAmount)
      .accountsPartial({
        user: payee1.publicKey,
        vaultState,
        vault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([payee1])
      .rpc();

    console.log("\nAuthorized payee withdrawal signature", tx);
  });

  // Test: Unauthorized user cannot withdraw from the vault
  it("Unauthorized user cannot withdraw", async () => {
    try {
      await program.methods
        .withdraw(new anchor.BN(0.1 * anchor.web3.LAMPORTS_PER_SOL))
        .accountsPartial({
          user: unauthorizedUser.publicKey,
          vaultState,
          vault,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([unauthorizedUser])
        .rpc();

      assert.fail("Expected error when unauthorized user tries to withdraw");
    } catch (error) {
      console.log("Successfully prevented unauthorized withdrawal");
      assert.equal(error.error.errorCode.code, "UnauthorizedPayee");
    }
  });

  // Test: Admin can remove a payee from the vault
  it("Admin can remove payee", async () => {
    const tx = await program.methods
      .removePayee(payee2.publicKey)
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
      })
      .rpc();

    console.log("\nRemoving payee transaction signature", tx);
  });

  // Test: Admin can schedule a payout for a payee
  it("Admin can schedule payout", async () => {
    const now = Math.floor(Date.now() / 1000);
    const startTime = now + 5; // Start in 5 seconds
    const interval = 10; // 10 seconds between payouts
    const amount = new anchor.BN(0.1 * anchor.web3.LAMPORTS_PER_SOL);

    const tx = await program.methods
      .schedulePayout(
        payee1.publicKey,
        amount,
        new anchor.BN(startTime),
        new anchor.BN(interval)
      )
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
      })
      .rpc();

    console.log("\nScheduling payout transaction signature", tx);
  });

  // Test: Payee cannot claim payout before the scheduled time
  it("Cannot claim payout before time", async () => {
    try {
      await program.methods
        .claimPayout()
        .accountsPartial({
          user: payee1.publicKey,
          vaultState,
          vault,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([payee1])
        .rpc();

      assert.fail("Should not be able to claim before scheduled time");
    } catch (error) {
      console.log("Successfully prevented early payout claim");
      assert.equal(error.error.errorCode.code, "PayoutTimeNotReached");
    }
  });

  // Test: Admin can cancel a payout schedule
  it("Admin can cancel payout schedule", async () => {
    const tx = await program.methods
      .cancelPayout(payee1.publicKey)
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
      })
      .rpc();

    console.log("\nCanceling payout schedule transaction signature", tx);
  });

  // Test: Payee cannot claim from a cancelled payout schedule
  it("Cannot claim from cancelled schedule", async () => {
    try {
      await program.methods
        .claimPayout()
        .accountsPartial({
          user: payee1.publicKey,
          vaultState,
          vault,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([payee1])
        .rpc();

      assert.fail("Should not be able to claim from cancelled schedule");
    } catch (error) {
      console.log("Successfully prevented claim from cancelled schedule");
      assert.equal(error.error.errorCode.code, "ScheduleNotFound");
    }
  });

  // Test: Admin can close the vault
  it("Close vault", async () => {
    const tx = await program.methods
      .close()
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
        vault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\nYour transaction signature", tx);
    console.log(
      "Your vault info",
      await provider.connection.getAccountInfo(vault)
    );
  });
});
