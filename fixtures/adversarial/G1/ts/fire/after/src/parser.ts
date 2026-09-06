export function parse(input: string): string[] {
{{weeder:ours}} HEAD
  return input.split(",");
{{weeder:separator}}
  return input.split(";");
{{weeder:theirs}} feature/split-on-semicolons
}
