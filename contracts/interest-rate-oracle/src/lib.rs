#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Vec,
};

const MAX_RATE_BPS: u32 = 100_000;

#[derive(Clone)]
#[contracttype]
pub struct Rate {
    pub rate_bps: u32,
    pub updated_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct ListingPrice {
    pub ask_price: i128,
    pub quantity: i128,
    pub recorded_at: u64,
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Admin,
    Rate(BytesN<32>),
    PriceHistory(BytesN<32>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracterror]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidRate = 3,
}

#[contract]
pub struct InterestRateOracle;

#[contractimpl]
impl InterestRateOracle {
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::Admin)
            .is_some()
        {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        Ok(())
    }
    pub fn set_rate(env: Env, invoice_id: BytesN<32>, rate_bps: u32) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        if rate_bps > MAX_RATE_BPS {
            return Err(Error::InvalidRate);
        }
        let rate = Rate {
            rate_bps,
            updated_at: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Rate(invoice_id.clone()), &rate);
        env.events()
            .publish((symbol_short!("rate"), invoice_id), rate_bps);
        Ok(())
    }
    pub fn get_rate(env: Env, invoice_id: BytesN<32>) -> Result<Rate, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Rate(invoice_id))
            .ok_or(Error::NotInitialized)
    }
    pub fn get_rate_bps(env: Env, invoice_id: BytesN<32>) -> Result<u32, Error> {
        Ok(Self::get_rate(env, invoice_id)?.rate_bps)
    }

    /// Record an ask price and retain the last 90 days of observations.
    pub fn record_listing_price(
        env: Env,
        invoice_id: BytesN<32>,
        ask_price: i128,
        quantity: i128,
    ) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        if ask_price <= 0 || quantity <= 0 {
            return Err(Error::InvalidRate);
        }
        let mut history: Vec<ListingPrice> = env
            .storage()
            .persistent()
            .get(&DataKey::PriceHistory(invoice_id.clone()))
            .unwrap_or(Vec::new(&env));
        history.push_back(ListingPrice {
            ask_price,
            quantity,
            recorded_at: env.ledger().timestamp(),
        });
        let cutoff = env.ledger().timestamp().saturating_sub(90 * 24 * 60 * 60);
        while history.len() > 0 && history.get(0).unwrap().recorded_at < cutoff {
            history.remove(0);
        }
        env.storage()
            .persistent()
            .set(&DataKey::PriceHistory(invoice_id.clone()), &history);
        env.events()
            .publish((symbol_short!("price"), invoice_id), (ask_price, quantity));
        Ok(())
    }

    pub fn get_price_history(env: Env, invoice_id: BytesN<32>) -> Vec<ListingPrice> {
        env.storage()
            .persistent()
            .get(&DataKey::PriceHistory(invoice_id))
            .unwrap_or(Vec::new(&env))
    }

    /// Return annualized implied yield in basis points using the latest ask,
    /// face value, and remaining tenor supplied by the caller.
    pub fn get_implied_yield(
        _env: Env,
        ask_price: i128,
        face_value: i128,
        remaining_seconds: u64,
    ) -> Result<i128, Error> {
        if ask_price <= 0 || face_value <= 0 || remaining_seconds == 0 {
            return Err(Error::InvalidRate);
        }
        let profit = face_value
            .checked_sub(ask_price)
            .ok_or(Error::InvalidRate)?;
        Ok(profit
            .checked_mul(10_000)
            .and_then(|v| v.checked_mul(365 * 24 * 60 * 60))
            .and_then(|v| v.checked_div(ask_price))
            .and_then(|v| v.checked_div(remaining_seconds as i128))
            .ok_or(Error::InvalidRate)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    #[test]
    fn admin_updates_invoice_rate() {
        let env = Env::default();
        let contract = env.register(InterestRateOracle, ());
        let client = InterestRateOracleClient::new(&env, &contract);
        let admin = Address::generate(&env);
        let invoice = BytesN::from_array(&env, &[1; 32]);
        client.initialize(&admin);
        client.set_rate(&invoice, &1_250);
        assert_eq!(client.get_rate_bps(&invoice), 1_250);
    }
}
