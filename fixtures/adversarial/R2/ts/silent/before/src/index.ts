import { greet } from "./greet";

export function main(): void {
  console.log(greet("world"));
}

export function formatRecord(fields: string[]): string {
  return fields.join(",");
}
