export function parse(input: string): number {
  const trimmed = input.trim();
  if (trimmed === "") {
    throw new Error("empty input");
  }
  return Number(trimmed);
}

export async function load(input: string): Promise<number> {
  return parse(input);
}

export function normalise(input: string): string {
  return input.trim();
}
