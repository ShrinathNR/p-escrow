use pinocchio::{account_info::AccountInfo, instruction::{Seed, Signer}, program_error::ProgramError, pubkey::{self}, ProgramResult};
use pinocchio_token::{instructions::{CloseAccount, Transfer}, state::TokenAccount};
use crate::Escrow;

pub struct Refund;

impl Refund{
    pub const DISCRIMINATOR : &u8 = &2;

    pub fn process(accounts: &[AccountInfo]) -> ProgramResult {

        let [maker, escrow, mint_a, mint_b, maker_ata_a, vault, _] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !maker.is_signer() {
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

        Transfer {
            from: vault,
            to: maker_ata_a,
            authority: escrow,
            amount: TokenAccount::from_account_info(vault)?.amount()
        }.invoke_signed(&signer)?;

        CloseAccount {
            account: vault,
            destination: maker,
            authority: escrow,
        }.invoke_signed(&signer)?;

        *maker.try_borrow_mut_lamports()? += *escrow.try_borrow_lamports()?;
        escrow.realloc(1, true)?;
        escrow.close()?;

        Ok(())
        
    }
}
