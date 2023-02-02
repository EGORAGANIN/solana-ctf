use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{get_authority, get_wallet_address, WalletInstruction};

// There's a mitigation for this bug in spl-token 3.1.1
// vendored_spl_token is an exact copy of spl-token 3.1.0, which doesn't have the mitigation yet
use vendored_spl_token as spl_token;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    mut instruction_data: &[u8],
) -> ProgramResult {
    match WalletInstruction::deserialize(&mut instruction_data)? {
        WalletInstruction::Initialize => initialize(program_id, accounts),
        WalletInstruction::Deposit { amount } => deposit(program_id, accounts, amount),
        WalletInstruction::Withdraw { amount } => withdraw(program_id, accounts, amount),
    }
}

fn initialize(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("init");
    let account_info_iter = &mut accounts.iter();
    let wallet_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    // @audit-issue (MEDIUM) - program expected spl_token program, but we can be substitute as fake program
    // It's not problem for initialize instruction, but may become a problem in the future
    // required add check assert_eq!(spl_token.key, spl_token::id());
    let spl_token = next_account_info(account_info_iter)?;

    // @audit-info - PDA -> owner, wallet_program
    let (wallet_address, wallet_seed) = get_wallet_address(owner.key, program_id);
    // @audit-info - authority single for program
    let (authority_address, _) = get_authority(program_id);
    let rent = Rent::from_account_info(rent_info)?;

    assert_eq!(wallet_info.key, &wallet_address); // @audit-info - exclude substitution wallet_info, because calculation PDA wallet_address
    assert_eq!(authority_info.key, &authority_address); // @audit-info - exclude substitution authority_info, because calculation PDA authority_address
    assert!(owner.is_signer, "owner must sign!"); // @audit-info - bind owner signature and wallet_info

    // @audit - when account create spl_token program owner wallet_address
    // fake program can close any account which owned, transfer rent funds to hacker
    invoke_signed(
        &system_instruction::create_account(
            &owner.key,
            &wallet_address,
            rent.minimum_balance(spl_token::state::Account::LEN),
            spl_token::state::Account::LEN as u64,
            &spl_token.key,
        ),
        &[owner.clone(), wallet_info.clone()],
        &[&[&owner.key.to_bytes(), &[wallet_seed]]],
    )?;

    // @audit - initialize_account not check token_program_id
    invoke(
        &spl_token::instruction::initialize_account(
            &spl_token.key,
            &wallet_address,
            mint.key,
            &authority_address,
        )
        .unwrap(),
        &[
            authority_info.clone(),
            wallet_info.clone(),
            mint.clone(),
            rent_info.clone(),
        ],
    )?;

    Ok(())
}

fn deposit(_program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    msg!("deposit {}", amount);
    let account_info_iter = &mut accounts.iter();
    let wallet_info = next_account_info(account_info_iter)?;
    let source_info = next_account_info(account_info_iter)?;
    let user_authority_info = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    // @audit-issue (MEDIUM) - program expected spl_token program, but we can be substitute as fake program
    // It's not problem for deposit instruction, but may become a problem in the future
    // required add check assert_eq!(spl_token.key, spl_token::id());
    let spl_token = next_account_info(account_info_iter)?;

    // @audit-issue (LOW) - direct read raw data is risk
    // recommended deserialize structure or check mint.owner == spl_token.key, Mint::LEN == mint.data.len()
    let decimals = mint.data.borrow()[44];

    // @audit - transfer_checked not check token_program_id
    invoke(
        &spl_token::instruction::transfer_checked(
            &spl_token.key,
            &source_info.key, // w
            mint.key, // r
            wallet_info.key, // w
            user_authority_info.key, // r, s
            &[],
            amount,
            decimals,
        )
        .unwrap(),
        &[
            wallet_info.clone(),
            source_info.clone(),
            user_authority_info.clone(),
            mint.clone(),
        ],
    )?;

    Ok(())
}

fn withdraw(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    msg!("withdraw {}", amount);
    let account_info_iter = &mut accounts.iter();
    let wallet_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let destination_info = next_account_info(account_info_iter)?;
    let mint = next_account_info(account_info_iter)?;
    let spl_token = next_account_info(account_info_iter)?;

    let (wallet_address, _) = get_wallet_address(owner_info.key, program_id);
    let (authority_address, authority_seed) = get_authority(program_id);

    assert_eq!(wallet_info.key, &wallet_address); // @audit-info - exclude substitution wallet_info, because calculation PDA wallet_address
    assert_eq!(authority_info.key, &authority_address); // @audit-info - exclude substitution authority_info, because calculation PDA authority_address
    assert!(owner_info.is_signer, "owner must sign!"); // @audit-info - bind owner signature and wallet_info

    // @audit-issue (LOW) - direct read raw data is risk
    // recommended deserialize structure or check mint.owner == spl_token.key, Mint::LEN == mint.data.len()
    let decimals = mint.data.borrow()[44];

    // @audit-info - we can't proxy transfer inside fake spl_token program,
    // because we must pass all accounts for transfer via real spl_token program
    // @audit - try changing owner authority_info inside fake program
    // @audit-issue (CRITICAL) - transfer tokens inside fake program using transfer instruction
    // required add check assert_eq!(spl_token.key, spl_token::id());
    invoke_signed(
        &spl_token::instruction::transfer_checked(
            &spl_token.key,
            &wallet_info.key, // w
            mint.key, // r
            destination_info.key, // w
            authority_info.key, // r, s
            &[],
            amount,
            decimals,
        )
        .unwrap(),
        &[
            wallet_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            mint.clone(),
        ],
        &[&[&[authority_seed]]],
    )?;

    Ok(())
}
