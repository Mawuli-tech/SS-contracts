//! Invoice Maturity Reminder contract — issue #641.
//!
//! Emits `MaturityReminder` events at 30d, 14d, 7d, and 1d before each
//! registered invoice matures so the backend can trigger investor notifications.
//! Each interval fires at most once per invoice (deduplicated by bitmask).

#![no_std]

mod errors;
mod events;
mod storage;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

pub use errors::Error;
pub use types::{Config, InvoiceRecord, ReminderStatus};

use types::{REMINDER_INTERVALS, SECS_PER_DAY};

#[contract]
pub struct InvoiceMaturityReminder;

#[contractimpl]
impl InvoiceMaturityReminder {
    /// Initialize the contract. The `admin` manages invoice registration;
    /// the `operator` (backend) is the only caller allowed to run
    /// `check_maturity_reminders`.
    pub fn initialize(env: Env, admin: Address, operator: Address) -> Result<(), Error> {
        admin.require_auth();
        if storage::get_config(&env).is_some() {
            return Err(Error::AlreadyInit);
        }
        storage::set_config(&env, &Config { admin, operator });
        Ok(())
    }

    /// Admin-only: register an invoice so it will be evaluated during
    /// `check_maturity_reminders`. `maturity_timestamp` must be in the future.
    /// `total_outstanding` is the amount (in stroops) included in reminder events.
    pub fn register_invoice(
        env: Env,
        admin: Address,
        invoice_id: Symbol,
        maturity_timestamp: u64,
        total_outstanding: i128,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config = storage::get_config(&env).ok_or(Error::NotInit)?;
        if config.admin != admin {
            return Err(Error::Unauthorized);
        }
        if maturity_timestamp <= env.ledger().timestamp() {
            return Err(Error::InvalidMaturity);
        }
        if storage::get_invoice(&env, &invoice_id).is_some() {
            return Err(Error::InvoiceAlreadyRegistered);
        }

        storage::set_invoice(&env, &invoice_id, &InvoiceRecord { maturity_timestamp, total_outstanding });

        let mut list = storage::get_invoice_list(&env);
        list.push_back(invoice_id);
        storage::set_invoice_list(&env, &list);

        Ok(())
    }

    /// Operator-only: iterate all registered invoices, check which reminder
    /// intervals have not yet been emitted, and emit `MaturityReminder` for
    /// each newly reached interval.
    ///
    /// Returns the number of reminder events emitted in this call.
    pub fn check_maturity_reminders(env: Env, operator: Address) -> Result<u32, Error> {
        operator.require_auth();
        let config = storage::get_config(&env).ok_or(Error::NotInit)?;
        if config.operator != operator {
            return Err(Error::Unauthorized);
        }

        let now = env.ledger().timestamp();
        let invoice_list = storage::get_invoice_list(&env);
        let mut emitted: u32 = 0;

        for invoice_id in invoice_list.iter() {
            let record = match storage::get_invoice(&env, &invoice_id) {
                Some(r) => r,
                None => continue,
            };

            // Skip invoices that have already matured — reminders are pre-maturity only.
            if now >= record.maturity_timestamp {
                continue;
            }

            let secs_remaining = record.maturity_timestamp - now;
            let days_remaining = secs_remaining / SECS_PER_DAY;

            let mut mask = storage::get_reminder_mask(&env, &invoice_id);

            for (interval_days, flag) in REMINDER_INTERVALS {
                // Interval is reached when days_remaining <= interval_days.
                if days_remaining <= interval_days && (mask & flag == 0) {
                    events::maturity_reminder(
                        &env,
                        invoice_id.clone(),
                        days_remaining,
                        record.total_outstanding,
                    );
                    mask |= flag;
                    emitted += 1;
                }
            }

            storage::set_reminder_mask(&env, &invoice_id, mask);
        }

        Ok(emitted)
    }

    /// Return the reminder status (which intervals have been emitted) for an
    /// invoice.
    pub fn get_reminder_status(env: Env, invoice_id: Symbol) -> Result<ReminderStatus, Error> {
        storage::get_invoice(&env, &invoice_id).ok_or(Error::InvoiceNotFound)?;
        let mask = storage::get_reminder_mask(&env, &invoice_id);
        Ok(ReminderStatus::from_mask(mask))
    }
}
