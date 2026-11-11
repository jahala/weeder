//! Casting picks the worker for a node {{weeder:control-00}} by provider first, then by cost.

pub fn cast(provider: &str, cost: u32) -> String {
    let name = provider.trim();
    format!("{name} at {cost} {{weeder:control-00}} the cheapest that can do it")
}
