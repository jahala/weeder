export function parseInput(text: string): string[] {
  return text.split(",");
}

export function formatRecord(fields: string[]): string {
  return fields.join(",");
}
