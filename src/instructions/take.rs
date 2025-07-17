use pinocchio::{account_info::AccountInfo, instruction::{Seed, Signer}, program_error::ProgramError, pubkey::{self}, sysvars::{rent::Rent, Sysvar}, ProgramResult};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{instructions::{CloseAccount, Transfer}, state::TokenAccount};
use pinocchio_associated_token_account::instructions::{CreateIdempotent as CreateATAIdempotent};

use crate::Escrow;

pub struct Take;

impl Take{
    pub const DISCRIMINATOR : &u8 = &1;

    pub fn process(accounts: &[AccountInfo]) -> ProgramResult {

        let [taker, maker, escrow, mint_a, mint_b, maker_ata_b, taker_ata_a, taker_ata_b, vault, system_program, token_program, _] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !taker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let esrow_binding = escrow.try_borrow_data()?;
        let escrow_data = Escrow::load(&esrow_binding)?;

        if mint_a.key().ne(&escrow_data.mint_a) { return Err(ProgramError::InvalidAccountData) }
        if mint_b.key().ne(&escrow_data.mint_b) { return Err(ProgramError::InvalidAccountData) }

        let derived_escrow_pubkey = pubkey::create_program_address(&[b"escrow", maker.key(), &escrow_data.seed.to_le_bytes()], &crate::ID)?;

        if escrow.key().ne(&derived_escrow_pubkey) { return Err(ProgramError::InvalidAccountData) }

        let seed_binding = escrow_data.seed.to_le_bytes();
        let bump_binding = [escrow_data.bump[0]];
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

        

        CreateATAIdempotent {
            funding_account: taker,
            account: maker_ata_b,
            wallet: maker,
            mint: mint_b,
            system_program,
            token_program
        }.invoke()?;

        CreateATAIdempotent {
            funding_account: taker,
            account: taker_ata_a,
            wallet: taker,
            mint: mint_a,
            system_program,
            token_program
        }.invoke()?;

        Transfer {
            from: taker_ata_b,
            to: maker_ata_b,
            authority: taker,
            amount: escrow_data.receive
        }.invoke()?;
        
        Transfer {
            from: vault,
            to: taker_ata_a,
            authority: escrow,
            amount: TokenAccount::from_account_info(vault)?.amount()
        }.invoke_signed(&signer)?;


        CloseAccount {
            account: vault,
            destination: taker,
            authority: escrow,
        }.invoke_signed(&signer)?;

        *taker.try_borrow_mut_lamports()? += *escrow.try_borrow_lamports()?;
        escrow.realloc(1, true)?;
        escrow.close()?;

        Ok(())
        
    }
}
