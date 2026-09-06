// Split the record on the separator the header names.
const todoLabel = "TODO: the queue shows this to whoever opens it";

export function parse(line: string): string[] {
  return line.split(",");
}

export function todoLabelFor(): string {
  return todoLabel;
}
