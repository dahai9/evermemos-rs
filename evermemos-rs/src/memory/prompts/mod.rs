pub mod en;
pub mod zh;

/// Current locale — can be overridden per-request in future.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    Zh,
}

impl Default for Locale {
    fn default() -> Self {
        Locale::En
    }
}
