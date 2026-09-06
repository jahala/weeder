export class SeparatorError extends Error {}

export function parse(input: string): string[] {
  if (input.length === 0) {
    throw new Error("input is empty");
  }
  if (input.includes(";;")) {
    throw new SeparatorError("stray separator");
  }
  return input.split(";");
}
