// TODO: split the record on the separator the header names
export function parse(line: string): string[] {
  return line.split(",");
}
