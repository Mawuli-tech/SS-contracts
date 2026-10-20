use soroban_sdk::{contracttype, Address, Symbol};

/// Reminder interval bitmask flags.
/// Each bit represents one interval; set = reminder already emitted for that interval.
pub const INTERVAL_30D: u32 = 0b0001;
pub const INTERVAL_14D: u32 = 0b0010;
pub const INTERVAL_7D:  u32 = 0b0100;
pub const INTERVAL_1D:  u32 = 0b1000;

/// (days_before_maturity, bitmask_flag) — evaluated in descending order so the
/// outermost interval is checked first.
pub const REMINDER_INTERVALS: [(u64, u32); 4] = [
    (30, INTERVAL_30D),
    (14, INTERVAL_14D),
    (7,  INTERVAL_7D),
    (1,  INTERVAL_1D),
];

pub const SECS_PER_DAY: u64 = 86_400;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageKey {
    /// Instance: global config.
    Config,
    /// Persistent: list of all registered invoice IDs.
    InvoiceList,
    /// Persistent: invoice record by ID.
    Invoice(Symbol),
    /// Persistent: bitmask of already-emitted reminder intervals per invoice.
    ReminderStatus(Symbol),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    /// Admin allowed to register invoices and update config.
    pub admin: Address,
    /// Operator (backend) allowed to call check_maturity_reminders.
    pub operator: Address,
}

/// Per-invoice data required to compute reminders.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvoiceRecord {
    /// Unix timestamp (ledger time) when the invoice matures.
    pub maturity_timestamp: u64,
    /// Total outstanding amount in stroops used in the MaturityReminder event.
    pub total_outstanding: i128,
}

/// Read-only view of which reminder intervals have been emitted for an invoice.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReminderStatus {
    pub days_30_emitted: bool,
    pub days_14_emitted: bool,
    pub days_7_emitted:  bool,
    pub days_1_emitted:  bool,
}

impl ReminderStatus {
    pub fn from_mask(mask: u32) -> Self {
        Self {
            days_30_emitted: mask & INTERVAL_30D != 0,
            days_14_emitted: mask & INTERVAL_14D != 0,
            days_7_emitted:  mask & INTERVAL_7D  != 0,
            days_1_emitted:  mask & INTERVAL_1D  != 0,
        }
    }
}
