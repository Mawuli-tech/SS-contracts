#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, BytesN, Env, Symbol,
};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Unlock(BytesN<32>),
    Emitted(BytesN<32>),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidDuration = 4,
    Locked = 5,
}

#[contract]
pub struct FractionTransferRestriction;

#[contractimpl]
impl FractionTransferRestriction {
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        Ok(())
    }

    pub fn set_lockup(
        env: Env,
        invoice_id: BytesN<32>,
        duration_seconds: u64,
    ) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        let unlock = env
            .ledger()
            .timestamp()
            .checked_add(duration_seconds)
            .ok_or(Error::InvalidDuration)?;
        env.storage()
            .persistent()
            .set(&DataKey::Unlock(invoice_id), &unlock);
        Ok(())
    }

    pub fn is_transferable(env: Env, invoice_id: BytesN<32>, _wallet: Address) -> (bool, u64) {
        let unlock = env
            .storage()
            .persistent()
            .get(&DataKey::Unlock(invoice_id))
            .unwrap_or(0);
        (unlock == 0 || env.ledger().timestamp() >= unlock, unlock)
    }

    /// Called by the fraction token before transfer/listing. Emits the unlock
    /// event exactly once, on the first eligible transfer.
    pub fn check_transfer(env: Env, invoice_id: BytesN<32>, wallet: Address) -> Result<(), Error> {
        let (allowed, unlock) = Self::is_transferable(env.clone(), invoice_id.clone(), wallet);
        if !allowed {
            return Err(Error::Locked);
        }
        if unlock != 0
            && !env
                .storage()
                .persistent()
                .has(&DataKey::Emitted(invoice_id.clone()))
        {
            env.storage()
                .persistent()
                .set(&DataKey::Emitted(invoice_id), &true);
            env.events()
                .publish((Symbol::new(&env, "LockupExpired"),), unlock);
        }
        Ok(())
    }
}
