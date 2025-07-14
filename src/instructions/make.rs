use pinocchio::{account_info::AccountInfo, instruction::{Seed, Signer}, program_error::ProgramError, pubkey::{self}, sysvars::{rent::Rent, Sysvar}, ProgramResult};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::Transfer;
use pinocchio_associated_token_account::instructions::Create as CreateATA;

use crate::{AccountCheck, Escrow, MintAccount};

pub struct Make;

impl Make{
    pub const DISCRIMINATOR : &u8 = &0;

    pub fn process(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

        let [maker, escrow, mint_a, mint_b, maker_ata_a, vault, system_program, token_program, _] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !maker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        MintAccount::check(mint_a)?;
        MintAccount::check(mint_b)?;

        let seed = u64::from_le_bytes(instruction_data[0..8].try_into().unwrap());
        let receive = u64::from_le_bytes(instruction_data[8..16].try_into().unwrap());
        let amount = u64::from_le_bytes(instruction_data[16..24].try_into().unwrap());

        if amount == 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        let (_, bump) = pubkey::find_program_address(&[b"escrow", maker.key(), &seed.to_le_bytes()], &crate::ID);

        let seed_binding = seed.to_le_bytes();
        let bump_binding = [bump];
        let escrow_seeds = [
            Seed::from(b"escrow"),
            Seed::from(maker.key().as_ref()),
            Seed::from(&seed_binding),
            Seed::from(&bump_binding),
        ];
        let signer = [Signer::from(&escrow_seeds)];


        CreateAccount{
            from: maker,
            to: escrow,
            lamports: Rent::get()?.minimum_balance(Escrow::LEN),
            space: Escrow::LEN as u64,
            owner: &crate::ID
        }.invoke_signed(&signer)?;

        let mut esrow_binding = escrow.try_borrow_mut_data()?;
        let escrow_data = Escrow::load_mut(&mut esrow_binding)?;

        escrow_data.set_inner(
            seed, 
            *maker.key(), 
            *mint_a.key(), 
            *mint_b.key(), 
            receive, 
            [bump]
        );

        CreateATA {
            funding_account: maker,
            account: vault,
            wallet: escrow,
            mint: mint_a,
            system_program,
            token_program
        }.invoke();



        Transfer {
            from: maker_ata_a,
            to: vault,
            authority: maker,
            amount
        }.invoke()?;



        Ok(())
        
    }
}
