export function parse(input: string): string[] {
  return input.length === 0 ? [] : input.split(",");
}
