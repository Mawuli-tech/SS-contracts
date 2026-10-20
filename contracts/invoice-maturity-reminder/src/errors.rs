use soroban_sdk::contracterror;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInit = 1,
    /// Contract has not been initialized.
    NotInit = 2,
    /// Caller is not the configured admin or operator.
    Unauthorized = 3,
    /// Invoice is already registered.
    InvoiceAlreadyRegistered = 4,
    /// No record found for the given invoice ID.
    InvoiceNotFound = 5,
    /// Maturity timestamp must be strictly in the future.
    InvalidMaturity = 6,
}
