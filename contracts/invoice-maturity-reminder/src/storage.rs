use soroban_sdk::{Env, Symbol, Vec};

use crate::types::{Config, InvoiceRecord, StorageKey};

pub const TTL_THRESHOLD: u32 = 120_960;
pub const MIN_TTL_EXTEND: u32 = 1_036_800;

fn bump_persistent(env: &Env, key: &StorageKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, TTL_THRESHOLD, MIN_TTL_EXTEND);
}

// ── Config ────────────────────────────────────────────────────────────────────

pub fn get_config(env: &Env) -> Option<Config> {
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD, MIN_TTL_EXTEND);
    env.storage().instance().get(&StorageKey::Config)
}

pub fn set_config(env: &Env, config: &Config) {
    env.storage().instance().set(&StorageKey::Config, config);
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD, MIN_TTL_EXTEND);
}

// ── Invoice list ──────────────────────────────────────────────────────────────

pub fn get_invoice_list(env: &Env) -> Vec<Symbol> {
    let key = StorageKey::InvoiceList;
    let list = env
        .storage()
        .persistent()
        .get::<_, Vec<Symbol>>(&key)
        .unwrap_or_else(|| Vec::new(env));
    if !list.is_empty() {
        bump_persistent(env, &key);
    }
    list
}

pub fn set_invoice_list(env: &Env, list: &Vec<Symbol>) {
    let key = StorageKey::InvoiceList;
    env.storage().persistent().set(&key, list);
    bump_persistent(env, &key);
}

// ── Invoice record ────────────────────────────────────────────────────────────

pub fn get_invoice(env: &Env, invoice_id: &Symbol) -> Option<InvoiceRecord> {
    let key = StorageKey::Invoice(invoice_id.clone());
    let data = env.storage().persistent().get(&key);
    if data.is_some() {
        bump_persistent(env, &key);
    }
    data
}

pub fn set_invoice(env: &Env, invoice_id: &Symbol, record: &InvoiceRecord) {
    let key = StorageKey::Invoice(invoice_id.clone());
    env.storage().persistent().set(&key, record);
    bump_persistent(env, &key);
}

// ── Reminder status bitmask ───────────────────────────────────────────────────

pub fn get_reminder_mask(env: &Env, invoice_id: &Symbol) -> u32 {
    let key = StorageKey::ReminderStatus(invoice_id.clone());
    let mask = env.storage().persistent().get::<_, u32>(&key).unwrap_or(0);
    if mask != 0 {
        bump_persistent(env, &key);
    }
    mask
}

pub fn set_reminder_mask(env: &Env, invoice_id: &Symbol, mask: u32) {
    let key = StorageKey::ReminderStatus(invoice_id.clone());
    env.storage().persistent().set(&key, &mask);
    bump_persistent(env, &key);
}
