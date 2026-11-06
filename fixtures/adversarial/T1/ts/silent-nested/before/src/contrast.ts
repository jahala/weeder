const ratios: Record<string, number> = {
  "light text paper": 12.6,
  "light muted paper": 5.1,
  "light edge paper": 3.4,
  "light focus paper": 4.2,
  "dark text paper": 13.1,
  "dark muted paper": 5.6,
  "dark edge paper": 3.2,
  "dark focus paper": 4.8,
};

export function contrast(theme: string, fg: string, bg: string): number {
  return ratios[`${theme} ${fg} ${bg}`] ?? 1;
}
