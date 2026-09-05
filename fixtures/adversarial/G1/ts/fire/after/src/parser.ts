export function parse(input: string): string[] {
{{weed:ours}} HEAD
  return input.split(",");
{{weed:separator}}
  return input.split(";");
{{weed:theirs}} feature/split-on-semicolons
}
