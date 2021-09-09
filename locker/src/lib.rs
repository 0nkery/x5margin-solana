use fixed::types::U64F64;
use parity_scale_codec::Decode;
use solana_api_types::{program::ProgramError, Instruction, Pubkey};
#[cfg(feature = "onchain")]
use solar::input::BpfProgramInput;
use solar::{
    input::{AccountSource, ProgramInput},
    math::Checked,
    prelude::AccountBackend,
    time::SolTimestamp,
    util::{ResultExt, pubkey_eq, timestamp_now},
    spl::WalletAccount,
    qlog,
};

pub mod error;
pub mod data;

#[macro_use]
extern crate parity_scale_codec;

#[macro_use]
extern crate solar_macros;

use crate::{
    error::Error,
    data::EntityKind,
};


pub type TokenAmount = Checked<u64>;
pub type TokenAmountF64 = Checked<U64F64>;

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
pub enum Method {
    CreateLock {
        unlock_date: SolTimestamp,
        amount: TokenAmount,
    },
    ReLock {
        unlock_date: SolTimestamp,
    },
    Withdraw {
        amount: TokenAmount,
    },
    Increment {
        amount: TokenAmount,
    },
    Split {
        amount: TokenAmount,
    },
    ChangeOwner {
        amount: TokenAmount,
    },
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct TokenLockState {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub program_authority: Pubkey,
    pub release_date: SolTimestamp,
}

pub struct TokenLock<B: AccountBackend> {
    account: B,
}

#[derive(Debug)]
pub struct CreateArgsAccounts<B: AccountBackend> {
    pub locker: B, //(empty, uninitialized)
    pub source_spl_token_wallet: B,
    pub source_authority: B, //(signed)
    pub spl_token_wallet_vault: WalletAccount<B>, //(authority = program authority)
    pub program_authority: B,
    pub owner_authority: B, //withdraw authority
}

impl<B: AccountBackend> CreateArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &mut locker,
            &source_spl_token_wallet,
            &source_authority,
            &spl_token_wallet_vault,
            &program_authority,
            &owner_authority,
        }

        Ok(Self {
            locker,
            source_spl_token_wallet,
            source_authority,
            spl_token_wallet_vault,
            program_authority,
            owner_authority,
        })
    }
}

#[derive(Debug)]
pub struct ReLockArgsAccounts<B: AccountBackend> {
    pub locker: B, //(empty, uninitialized)
    pub owner_authority: B, //withdraw authority
}

impl<B: AccountBackend> ReLockArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &mut locker,
            &owner_authority,
        }

        Ok(Self {
            locker,
            owner_authority,
        })
    }
}

#[derive(Debug)]
pub struct WithdrawArgsAccounts<B: AccountBackend> {
    pub locker: B,
    pub spl_token_wallet_vault: WalletAccount<B>,
    pub destination_spl_token_wallet: B,
    pub program_authority: B,
    pub owner_authority: B,
}

impl<B: AccountBackend> WithdrawArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &mut locker,
            &spl_token_wallet_vault,
            &destination_spl_token_wallet,
            &program_authority,
            &owner_authority,
        }

        Ok(Self {
            locker,
            spl_token_wallet_vault,
            destination_spl_token_wallet,
            program_authority,
            owner_authority,
        })
    }
}

#[derive(Debug)]
pub struct IncrementArgsAccounts<B: AccountBackend> {
    pub locker: B, 
    pub spl_token_wallet_vault: WalletAccount<B>,
    pub source_spl_token_wallet: B,
    pub source_authority: B, 
}

impl<B: AccountBackend> IncrementArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &locker,
            &spl_token_wallet_vault,
            &source_spl_token_wallet,
            &source_authority,
        }

        Ok(Self {
            locker,
            spl_token_wallet_vault,
            source_spl_token_wallet,
            source_authority,
        })
    }
}

#[derive(Debug)]
pub struct SplitArgsAccounts<B: AccountBackend> {
    pub source_locker: B,
    pub new_locker: B, //(empty, uninitialized)
    pub source_spl_token_wallet_vault: WalletAccount<B>,
    pub new_spl_token_vault: WalletAccount<B>,
}

impl<B: AccountBackend> SplitArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &source_locker,
            &mut new_locker,
            &source_spl_token_wallet_vault,
            &new_spl_token_wallet_vault,
        }

        Ok(Self {
            source_locker,
            new_locker,
            spl_token_wallet_vault,
            source_spl_token_wallet_vault,
            new_spl_token_wallet_vault,
        })
    }
}

#[derive(Debug)]
pub struct ChangeOwnerArgsAccounts<B: AccountBackend> {
    pub locker: B,
    pub source_owner_authority: B,
    pub new_owner_authority: B,
}

impl<B: AccountBackend> ChangeOwnerArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &locker,
            &source_owner_authority,
            &new_owner_authority,
        }

        Ok(Self {
            locker,
            source_owner_authority,
            new_owner_authority,
        })
    }
}

pub const HEADER_RESERVED: usize = 96;

impl<B: AccountBackend> TokenLock<B> {

    pub fn raw_any(program_id: &Pubkey, account: B) -> Result<Self, Error> {
        let size = account.data().len();

        if size < HEADER_RESERVED || !T::is_valid_size(size - HEADER_RESERVED) {
            return Err(Error::InvalidData);
        }

        // require that account data is aligned on a 16-byte boundary
        // this is mostly important for offchain purposes
        if (account.data().as_ptr()) as usize % 16 != 0 {
            return Err(Error::InvalidAlignment);
        }

        let locker = Self {
            account,
        };

        if locker.account.owner() != program_id {
            return Err(Error::InvalidOwner);
        }

        // require all entities to be rent-exempt to be valid
        if B::Env::supports_syscalls() && !locker.is_rent_exempt(&Rent::get().bpf_unwrap()) {
            return Err(Error::NotRentExempt);
        };

        Ok(locker)
    }

    pub fn account(&self) -> &B {
        &self.account
    }

    /// Create a new locker.
    ///
    /// Account inputs:
    /// Locker (empty, uninitialized)
    /// SPL Token Wallet source
    /// Source Authority (signed)
    /// SPL Token Wallet vault (authority = program authority)
    /// Program Authority
    /// Owner (withdraw authority)
    pub fn create<S: AccountSource<B>>(
        input: S,
        unlock_date: SolTimestamp,
        amount: TokenAmount,
    ) -> Result<(), ProgramError> {

        let CreateArgsAccounts {
            locker,
            source_spl_token_wallet,
            source_authority,
            spl_token_wallet_vault,
            program_authority,
            owner_authority,
        } = CreateArgsAccounts::from_program_input(input)?;

        let mut new_locker = new TokenLock {
            account: locker,
        };

        entity.owner = *owner_authority.key();
        entity.mint = source_spl_token_wallet.mint();
        entity.vault = *spl_token_wallet_vault;
        entity.program_authority = *program_authority.key();
        entity.release_date = unlock_date;

        let id = entity.allocator.allocate_id();
        let entity_key = *entity.account().key();
        let header = entity.header_mut();
        header.kind = EntityKind::Locker;
        header.id = id;
        header.parent_id = id;
        header.root = entity_key;

        let expected_program_authority = Pubkey::create_program_address(
            &[
                entity.account().key().as_ref(),
                owner_authority.key().as_ref(),
            ],
            input.program_id(),
        )
        .bpf_expect("couldn't derive program authority");

        if !pubkey_eq(program_authority.key(), &expected_program_authority) {
            qlog!("provided program authority does not match expected authority");
            return Err(Error::InvalidAuthority);
        }

        if !pubkey_eq(spl_token_wallet_vault.authority(), &expected_program_authority) {
            qlog!("spl token wallet vault authority does not match program authority");
            return Err(Error::InvalidAuthority);
        }

        let now = timestamp_now();

        if entity.release_date <= now {
            qlog!("can`t initialize new locker with invalid unlock date");
            return Err(Error::InvalidData);
        }

        Ok(())
    }

    /// Returns 4 instructions:
    /// SystemProgram::CreateAccount (locker vault)
    /// SplToken::Initialize (locker vault)
    /// SystemProgram::Create (locker)
    /// Locker::Create (locker)
    pub fn create_instruction(
        locker: Pubkey,
        owner: Pubkey,
        source_wallet: Pubkey,
        source_mint: Pubkey,
        source_authority: Pubkey,
    ) -> [Instruction; 4] {

        let mut instructions = vec![];
        todo!();

        return instructions;
    }

    /// Relocks an existing locker with a new unlock date.
    ///
    /// Input accounts:
    /// Locker
    /// Locker Owner (signed)
    pub fn relock<S: AccountSource<B>>(
        input: S,
        unlock_date: SolTimestamp,
    ) -> Result<(), ProgramError> {

        let ReLockArgsAccounts {
            locker,
            owner_authority,
        } = ReLockArgsAccounts::from_program_input(input)?;

        if !pubkey_eq(locker.owner, owner_authority.key()) {
            qlog!("provided program authority does not match expected authority");
            return Err(Error::InvalidAuthority);
        }

        if unlock_date <= locker.release_date {
            qlog!("can`t initialize new locker with invalid unlock date");
            return Err(Error::InvalidData);
        }

        locker.release_date = unlock_date;

        Ok(())
    }

    /// Withdraw funds from locker.
    /// Input accounts:
    /// Locker
    /// SPL Token Wallet vault
    /// SPL Token Wallet destination
    /// Program Authority
    /// Owner (signed)
    pub fn withdraw<S: AccountSource<B>>(
        input: S,
        amount: TokenAmount,
    ) -> Result<(), ProgramError> {

        let WithdrawArgsAccounts {
            locker,
            spl_token_wallet_vault,
            destination_spl_token_wallet,
            program_authority,
            owner_authority,
        } = WithdrawArgsAccounts::from_program_input(input)?;

        todo!()
    }

    /// Add funds to locker
    ///
    /// Input accounts:
    /// Locker
    /// SPL Token Wallet vault
    /// SPL Token Wallet source
    /// Source Authority
    pub fn increment<S: AccountSource<B>>(
        input: S,
        amount: TokenAmount,
    ) -> Result<(), ProgramError> {

        let IncrementArgsAccounts {
            locker,
            spl_token_wallet_vault,
            source_spl_token_wallet,
            source_authority,
        } = IncrementArgsAccounts::from_program_input(input)?;

        todo!()
    }

    /// Split locker
    ///
    /// Input accounts:
    /// Source Locker
    /// New Locker
    /// Program Authority
    /// SPL Token Vault (Source Locker)
    /// SPL Token Vault (New Locker)
    pub fn split<S: AccountSource<B>>(input: S, amount: TokenAmount) -> Result<(), ProgramError> {
        
        let SplitArgsAccounts {
            source_locker,
            new_locker,
            source_spl_token_wallet_vault,
            new_spl_token_wallet_vault,
        } = SplitArgsAccounts::from_program_input(input)?;

        todo!()
    }

    /// Change locker owner
    ///
    /// Input accounts:
    /// Locker
    /// Owner (signed)
    /// New Owner
    pub fn change_owner<S: AccountSource<B>>(
        input: S,
        amount: TokenAmount,
    ) -> Result<(), ProgramError> {

        let ChangeOwnerArgsAccounts {
            locker,
            source_owner_authority,
            new_owner_authority,
        } = ChangeOwnerArgsAccounts::from_program_input(input)?;

        todo!()
    }
}

#[cfg(feature = "onchain")]
pub fn main(mut input: BpfProgramInput) -> Result<(), ProgramError> {
    let mut data = input.data();
    let method = Method::decode(&mut data)
        .ok()
        .bpf_expect("couldn't parse method");

    match method {
        Method::CreateLock {
            unlock_date,
            amount,
        } => TokenLock::create(input, unlock_date, amount).bpf_unwrap(),
        Method::ReLock {
            unlock_date,
        } => TokenLock::relock(input, unlock_date).bpf_unwra(),
        Method::Withdraw {
            amount,
        } => withdraw(input, amount).bpf_unwra(),
        Method::Increment {
            amount,
        } => increment(input, amount).bpf_unwra(),
        Method::Split {
            amount,
        } => split(input, amount).bpf_unwra(),
        Method::ChangeOwner {
            amount,
        } => change_owner(input, amount).bpf_unwra(),
    }

    Ok(())
}

#[cfg(feature = "onchain")]
#[cfg(test)]
mod test {

    #[tokio::test]
    async fn init_test() -> anyhow::Result<()> {

        let mut program_test = ProgramTest::default();
        let program_id = Pubkey::new_unique();

        program_test.add_program(
            "locker",
            program_id,
            Some(|a, b, c| {
                builtin_process_instruction(wrapped_entrypoint::<super::Program>, a, b, c)
            }),
        );

        let locker_key = Keypair::new();
        let locker_owner_key = Keypair::new();

        let mut salt: u64 = 0;
        let locker_program_authority = loop {
            let locker_program_authority = Pubkey::create_program_address(
                &[
                    locker_key.pubkey().as_ref(),
                    locker_owner_key.pubkey().as_ref(),
                ],
                &program_id,
            );

            match locker_program_authority {
                Some(s) => break s,
                None => {
                    salt += 1;
                }
            }
        };

        let (mut client, payer, hash) = program_test.start().await;

        todo!();

        Ok(())
    }
}
