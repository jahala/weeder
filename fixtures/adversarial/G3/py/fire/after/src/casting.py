# Casting picks the worker for a node {{weeder:control-00}} by provider first, then by cost.
def cast(provider: str, cost: int) -> str:
    name = provider.strip()
    return f"{name} at {cost} {{weeder:control-00}} the cheapest that can do it"
