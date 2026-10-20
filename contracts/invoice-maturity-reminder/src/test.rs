#![cfg(test)]

use soroban_sdk::{testutils::Ledger, Env, Symbol};

use crate::{InvoiceMaturityReminder, InvoiceMaturityReminderClient};
use crate::errors::Error;
use crate::types::{SECS_PER_DAY, INTERVAL_30D, INTERVAL_14D, INTERVAL_7D, INTERVAL_1D};

fn setup() -> (Env, InvoiceMaturityReminderClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, InvoiceMaturityReminder);
    let client = InvoiceMaturityReminderClient::new(&env, &contract_id);
    (env, client)
}

fn admin(env: &Env) -> soroban_sdk::Address {
    soroban_sdk::Address::generate(env)
}

fn operator(env: &Env) -> soroban_sdk::Address {
    soroban_sdk::Address::generate(env)
}

fn inv(env: &Env, name: &str) -> Symbol {
    Symbol::new(env, name)
}

// ── Initialization ────────────────────────────────────────────────────────────

#[test]
fn test_initialize_ok() {
    let (env, client) = setup();
    let a = admin(&env);
    let op = operator(&env);
    client.initialize(&a, &op);
}

#[test]
fn test_initialize_twice_rejected() {
    let (env, client) = setup();
    let a = admin(&env);
    let op = operator(&env);
    client.initialize(&a, &op);
    let err = client.try_initialize(&a, &op).unwrap_err().unwrap();
    assert_eq!(err, Error::AlreadyInit);
}

// ── Registration ──────────────────────────────────────────────────────────────

#[test]
fn test_register_invoice_ok() {
    let (env, client) = setup();
    let a = admin(&env);
    let op = operator(&env);
    client.initialize(&a, &op);

    let now = env.ledger().timestamp();
    client.register_invoice(&a, &inv(&env, "INV001"), &(now + 60 * SECS_PER_DAY), &1_000_000);
}

#[test]
fn test_register_duplicate_rejected() {
    let (env, client) = setup();
    let a = admin(&env);
    let op = operator(&env);
    client.initialize(&a, &op);

    let now = env.ledger().timestamp();
    let id = inv(&env, "INV001");
    client.register_invoice(&a, &id, &(now + 60 * SECS_PER_DAY), &1_000_000);
    let err = client.try_register_invoice(&a, &id, &(now + 60 * SECS_PER_DAY), &1_000_000)
        .unwrap_err().unwrap();
    assert_eq!(err, Error::InvoiceAlreadyRegistered);
}

#[test]
fn test_register_past_maturity_rejected() {
    let (env, client) = setup();
    let a = admin(&env);
    let op = operator(&env);
    client.initialize(&a, &op);

    let now = env.ledger().timestamp();
    let err = client
        .try_register_invoice(&a, &inv(&env, "INV002"), &(now - 1), &500_000)
        .unwrap_err().unwrap();
    assert_eq!(err, Error::InvalidMaturity);
}

// ── No reminder before threshold ──────────────────────────────────────────────

#[test]
fn test_no_reminder_outside_window() {
    let (env, client) = setup();
    let a = admin(&env);
    let op = operator(&env);
    client.initialize(&a, &op);

    let now = env.ledger().timestamp();
    // Invoice matures in 45 days — outside all four windows
    client.register_invoice(&a, &inv(&env, "INV_FAR"), &(now + 45 * SECS_PER_DAY), &1_000_000);

    let fired = client.check_maturity_reminders(&op);
    assert_eq!(fired, 0);

    let status = client.get_reminder_status(&inv(&env, "INV_FAR"));
    assert!(!status.days_30_emitted);
    assert!(!status.days_14_emitted);
    assert!(!status.days_7_emitted);
    assert!(!status.days_1_emitted);
}

// ── Reminder fired at 30d ─────────────────────────────────────────────────────

#[test]
fn test_reminder_fires_at_30_days() {
    let (env, client) = setup();
    let a = admin(&env);
    let op = operator(&env);
    client.initialize(&a, &op);

    let now = env.ledger().timestamp();
    // Matures in exactly 30 days (within the 30-day window)
    client.register_invoice(&a, &inv(&env, "INV30"), &(now + 30 * SECS_PER_DAY), &2_000_000);

    let fired = client.check_maturity_reminders(&op);
    assert_eq!(fired, 1);

    let status = client.get_reminder_status(&inv(&env, "INV30"));
    assert!(status.days_30_emitted);
    assert!(!status.days_14_emitted);
}

// ── Duplicate suppression ─────────────────────────────────────────────────────

#[test]
fn test_duplicate_reminder_suppressed() {
    let (env, client) = setup();
    let a = admin(&env);
    let op = operator(&env);
    client.initialize(&a, &op);

    let now = env.ledger().timestamp();
    client.register_invoice(&a, &inv(&env, "INV_DUP"), &(now + 7 * SECS_PER_DAY), &500_000);

    // First call — fires 30d, 14d, and 7d intervals
    let first = client.check_maturity_reminders(&op);
    assert_eq!(first, 3);

    // Second call — all intervals already emitted, nothing fires
    let second = client.check_maturity_reminders(&op);
    assert_eq!(second, 0);
}

// ── All four intervals fire for a 1d invoice ─────────────────────────────────

#[test]
fn test_all_intervals_fire_at_1_day() {
    let (env, client) = setup();
    let a = admin(&env);
    let op = operator(&env);
    client.initialize(&a, &op);

    let now = env.ledger().timestamp();
    // 1 day remaining — all four thresholds (30, 14, 7, 1) are satisfied
    client.register_invoice(&a, &inv(&env, "INV1D"), &(now + SECS_PER_DAY), &3_000_000);

    let fired = client.check_maturity_reminders(&op);
    assert_eq!(fired, 4);

    let status = client.get_reminder_status(&inv(&env, "INV1D"));
    assert!(status.days_30_emitted);
    assert!(status.days_14_emitted);
    assert!(status.days_7_emitted);
    assert!(status.days_1_emitted);
}

// ── Matured invoice skipped ───────────────────────────────────────────────────

#[test]
fn test_matured_invoice_not_reminded() {
    let (env, client) = setup();
    let a = admin(&env);
    let op = operator(&env);
    client.initialize(&a, &op);

    // Register invoice, then advance ledger past maturity
    let now = env.ledger().timestamp();
    client.register_invoice(&a, &inv(&env, "INV_PAST"), &(now + SECS_PER_DAY), &1_000_000);

    env.ledger().with_mut(|l| l.timestamp = now + 2 * SECS_PER_DAY);

    let fired = client.check_maturity_reminders(&op);
    assert_eq!(fired, 0);
}

// ── Unauthorized operator rejected ───────────────────────────────────────────

#[test]
fn test_unauthorized_operator_rejected() {
    let (env, client) = setup();
    let a = admin(&env);
    let op = operator(&env);
    client.initialize(&a, &op);

    let intruder = soroban_sdk::Address::generate(&env);
    let err = client.try_check_maturity_reminders(&intruder).unwrap_err().unwrap();
    assert_eq!(err, Error::Unauthorized);
}

// ── get_reminder_status unknown invoice ───────────────────────────────────────

#[test]
fn test_get_reminder_status_unknown_invoice() {
    let (env, client) = setup();
    let a = admin(&env);
    let op = operator(&env);
    client.initialize(&a, &op);

    let err = client.try_get_reminder_status(&inv(&env, "GHOST")).unwrap_err().unwrap();
    assert_eq!(err, Error::InvoiceNotFound);
}

// ── Interval flags are correct ────────────────────────────────────────────────

#[test]
fn test_interval_flag_values() {
    assert_eq!(INTERVAL_30D, 0b0001);
    assert_eq!(INTERVAL_14D, 0b0010);
    assert_eq!(INTERVAL_7D,  0b0100);
    assert_eq!(INTERVAL_1D,  0b1000);
    // All four set
    assert_eq!(INTERVAL_30D | INTERVAL_14D | INTERVAL_7D | INTERVAL_1D, 0b1111);
}
