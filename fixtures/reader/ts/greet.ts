import { formatName } from "./name";

export function greet(name: string): string {
  return `hello ${formatName(name)}`;
}
