// Casting picks the worker for a node — by provider first, then by cost.
export function cast(provider: string, cost: number): string {
  return `${provider} at ${cost} — the cheapest that can do it`;
}
