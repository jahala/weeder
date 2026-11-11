# Casting picks the worker for a node — by provider first, then by cost.
def cast(provider: str, cost: int) -> str:
    return f"{provider} at {cost} — the cheapest that can do it"
