import { execFileSync } from "node:child_process";

export function read(path: string): string {
  return execFileSync("git", ["show", path]).toString();
}
