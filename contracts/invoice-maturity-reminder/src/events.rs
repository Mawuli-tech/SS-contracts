#![allow(deprecated)]

use soroban_sdk::{Env, Symbol};

/// Emitted by check_maturity_reminders when an invoice enters a reminder window.
/// Includes days_remaining and total_outstanding for the backend notification layer.
pub fn maturity_reminder(
    env: &Env,
    invoice_id: Symbol,
    days_remaining: u64,
    total_outstanding: i128,
) {
    env.events().publish(
        (Symbol::new(env, "MaturityReminder"),),
        (invoice_id, days_remaining, total_outstanding),
    );
}
