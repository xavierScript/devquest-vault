import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DevquestVault } from "../target/types/devquest_vault";
import { Keypair } from "@solana/web3.js";
import { assert } from "chai";

describe("devquest-vault", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.DevquestVault as Program<DevquestVault>;

  // Create test payees
  const payee1 = Keypair.generate();
  const payee2 = Keypair.generate();
  const unauthorizedUser = Keypair.generate();

  // Admin will be the provider's wallet
  const admin = provider.wallet.publicKey;

  const vaultState = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("state"), admin.toBytes()],
    program.programId
  )[0];
  const vault = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), vaultState.toBytes()],
    program.programId
  )[0];

  // Fund the test accounts
  before(async () => {
    // Fund payee1 for transaction fees
    const fundTx = await provider.connection.requestAirdrop(
      payee1.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(fundTx);
  });

  it("Is initialized!", async () => {
    // Add your test here.
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

    // Verify vault state
    const vaultStateAccount = await program.account.vaultState.fetch(
      vaultState
    );
    console.log(
      "Current payees:",
      vaultStateAccount.payees.map((p) => p.toString())
    );
  });

  it("Only admin can add payees", async () => {
    try {
      // Try to add a payee using unauthorized user
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

  it("Authorized payee can withdraw", async () => {
    // First deposit some SOL
    await program.methods
      .deposit(new anchor.BN(2 * anchor.web3.LAMPORTS_PER_SOL))
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
        vault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    // Payee1 tries to withdraw
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

  it("Admin can remove payee", async () => {
    const tx = await program.methods
      .removePayee(payee2.publicKey)
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
      })
      .rpc();

    console.log("\nRemoving payee transaction signature", tx);

    // Verify payee was removed
    const vaultStateAccount = await program.account.vaultState.fetch(
      vaultState
    );
    console.log(
      "Remaining payees:",
      vaultStateAccount.payees.map((p) => p.toString())
    );
    assert(
      !vaultStateAccount.payees.some((p) => p.equals(payee2.publicKey)),
      "Payee2 should have been removed"
    );
  });

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
