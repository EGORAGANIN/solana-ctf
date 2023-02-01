use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    entrypoint,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

#[derive(Debug, BorshDeserialize, BorshSerialize)]

pub enum WalletInstruction {
    /// Initialize a Personal Savings Wallet
    ///
    /// Passed accounts:
    ///
    /// (1) Wallet account
    /// (2) Vault accounts
    /// (3) Authority
    /// (4) Rent sysvar
    /// (5) System program
    Initialize,
    /// Deposit
    ///
    /// Passed accounts:
    ///
    /// (1) Wallet account
    /// (2) Vault accounts
    /// (3) Money Source
    Deposit { amount: u64 },
    /// Withdraw from Wallet
    ///
    /// Passed accounts:
    ///
    /// (1) Wallet account
    /// (2) Vault accounts
    /// (3) Authority
    /// (4) Target Wallet account
    Withdraw { amount: u64 },
}

// @audit-ok - Structure deserialization by layout from data.
// Contract has single state account, so account substitution is not possible.
// @audit-issue (INFO) - recommended using ID/discriminator for differ layout.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Wallet {
    pub authority: Pubkey,
    pub vault: Pubkey,
}

pub const WALLET_LEN: u64 = 32 + 32;

pub mod processor;
use processor::process_instruction;
entrypoint!(process_instruction);

pub fn get_wallet_address(authority: Pubkey, wallet_program: Pubkey) -> Pubkey {
    let (wallet_address, _) =
        Pubkey::find_program_address(&[&authority.to_bytes()], &wallet_program);
    wallet_address
}

pub fn get_vault_address(authority: Pubkey, wallet_program: Pubkey) -> Pubkey {
    let (vault_address, _) = Pubkey::find_program_address(
        &[&authority.to_bytes(), &"VAULT".as_bytes()],
        &wallet_program,
    );
    vault_address
}

pub fn initialize(wallet_program: Pubkey, authority_address: Pubkey) -> Instruction {
    let wallet_address = get_wallet_address(authority_address, wallet_program);
    let vault_address = get_vault_address(authority_address, wallet_program);
    Instruction {
        program_id: wallet_program,
        accounts: vec![
            AccountMeta::new(wallet_address, false),
            AccountMeta::new(vault_address, false),
            AccountMeta::new(authority_address, true),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: WalletInstruction::Initialize.try_to_vec().unwrap(),
    }
}

pub fn deposit(
    wallet_program: Pubkey,
    authority_address: Pubkey,
    source: Pubkey,
    amount: u64,
) -> Instruction {
    let wallet_address = get_wallet_address(authority_address, wallet_program);
    let vault_address = get_vault_address(authority_address, wallet_program);
    Instruction {
        program_id: wallet_program,
        accounts: vec![
            AccountMeta::new(wallet_address, false),
            AccountMeta::new(vault_address, false),
            AccountMeta::new(source, true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: WalletInstruction::Deposit { amount }.try_to_vec().unwrap(),
    }
}

pub fn withdraw(
    wallet_program: Pubkey,
    authority_address: Pubkey,
    destination: Pubkey,
    amount: u64,
) -> Instruction {
    let wallet_address = get_wallet_address(authority_address, wallet_program);
    let vault_address = get_vault_address(authority_address, wallet_program);
    Instruction {
        program_id: wallet_program,
        accounts: vec![
            AccountMeta::new(wallet_address, false),
            AccountMeta::new(vault_address, false),
            AccountMeta::new(authority_address, true),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: WalletInstruction::Withdraw { amount }.try_to_vec().unwrap(),
    }
}
