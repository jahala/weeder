//! Casting picks the worker for a node — by provider first, then by cost.

pub fn cast(provider: &str, cost: u32) -> String {
    format!("{provider} at {cost} — the cheapest that can do it")
}
