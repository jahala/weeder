// Casting picks the worker for a node {{weeder:control-00}} by provider first, then by cost.
export function cast(provider: string, cost: number): string {
  const name = provider.trim();
  return `${name} at ${cost} {{weeder:control-00}} the cheapest that can do it`;
}
