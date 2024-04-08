import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { StakingRewards } from "../target/types/staking_rewards";
import { expect } from 'chai';
import { PublicKey, SystemProgram } from '@solana/web3.js';
import { BN } from "bn.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

describe("staking-rewards", () => {
  // Configure the client to use the local cluster.
  const provider =  anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.StakingRewards as Program<StakingRewards>;
  let stakingRewardsPublicKey;

  it("Initializes the staking rewards program", async () => {
    // Generate a new Keypair for the owner of the staking rewards account.
    const stakingRewards = anchor.web3.Keypair.generate();

// Initialize arguments
  const rewardRate = new BN(10); 
  const duration = new BN(86400); //  duration in seconds

  stakingRewardsPublicKey = stakingRewards.publicKey;
        
  // stakingToken and rewardsToken mint accounts
  const stakingToken = new PublicKey('GmT5u7pUjTSb2P6oNNtsmFt3EgvCtoPNNn9KKh4GhxAd');
  const rewardsToken = new PublicKey('FmecqzBvahi6QCrxySXgErEk2s2PB8wfEBQg3sfiYJJS');

  const tx = await program.methods.initialize(new BN(10), new BN(86400))
  .accounts({
    stakingRewards: stakingRewards.publicKey,
    owner: provider.wallet.publicKey,
    stakingToken: stakingToken,
    rewardsToken: rewardsToken, 
    systemProgram: SystemProgram.programId, 
  })
  .signers([stakingRewards]) 
  .rpc();

  const stakingRewardsState = await program.account.stakingRewardsState.fetch(stakingRewards.publicKey);
        // Assertions
        expect(stakingRewardsState.owner.toBase58()).to.equal(provider.wallet.publicKey.toBase58());
        expect(stakingRewardsState.stakingToken.toBase58()).to.equal(stakingToken.toBase58());
        expect(stakingRewardsState.rewardsToken.toBase58()).to.equal(rewardsToken.toBase58());
        expect(stakingRewardsState.duration.eq(duration)).to.be.true;
        expect(stakingRewardsState.rewardRate.eq(rewardRate)).to.be.true;
        expect(stakingRewardsState.totalSupply.toNumber()).to.equal(0);

    console.log("Your transaction signature", tx);
  });

//staker account : 46D4a7BC5J4fDkzCGRThs168F2XryB7TRWgDBfEquVAH
//pool token account : 913LX41gd6Lyzy37z46hdz1GMqaaghtbo8cUJBMXEUUp

it("Stakes tokens successfully", async () => {

  if (!stakingRewardsPublicKey) {
    throw new Error("stakingRewardsPublicKey not set. Initialize test may have failed.");
}

  const stakerTokenAccount = new PublicKey('46D4a7BC5J4fDkzCGRThs168F2XryB7TRWgDBfEquVAH'); // The staker's token account holding the staking tokens.
  const amountToStake = new BN(1000); // Amount to stake

  const poolTokenAccount = new PublicKey('913LX41gd6Lyzy37z46hdz1GMqaaghtbo8cUJBMXEUUp');

      // Before the stake operation, fetch the initial balances
      const stakerInitialBalance = await provider.connection.getTokenAccountBalance(stakerTokenAccount);

      const poolInitialBalance = await provider.connection.getTokenAccountBalance(poolTokenAccount);

      const stakerState = anchor.web3.Keypair.generate();

  // Execute the stake function
  await program.methods.stake(amountToStake)
    .accounts({
      stakerTokenAccount: stakerTokenAccount,
      poolTokenAccount: poolTokenAccount,
      staker: provider.wallet.publicKey, // This should be the state account for this specific staker.
      stakingRewards: stakingRewardsPublicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
      stakerState: stakerState.publicKey
    })
    .rpc();
      
    // Fetch updated balances
     const stakerFinalBalance = await provider.connection.getTokenAccountBalance(stakerTokenAccount);

     const poolFinalBalance = await provider.connection.getTokenAccountBalance(poolTokenAccount);

    
     // Convert balances to BN
    const stakerInitialBalanceBN = new anchor.BN(stakerInitialBalance.value.amount);
    const stakerFinalBalanceBN = new anchor.BN(stakerFinalBalance.value.amount);
    const poolInitialBalanceBN = new anchor.BN(poolInitialBalance.value.amount);
    const poolFinalBalanceBN = new anchor.BN(poolFinalBalance.value.amount);

    // Assertions
    // Check if the staker's balance decreased by the stake amount
    expect(stakerFinalBalanceBN).to.eql(stakerInitialBalanceBN.sub(amountToStake));

    // Check if the pool's balance increased by the stake amount
    expect(poolFinalBalanceBN).to.eql(poolInitialBalanceBN.add(amountToStake));




});
});