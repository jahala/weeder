import { spriteName } from "./sprites";

export function atlas(count: number): string[] {
  return Array.from({ length: count }, (_, index) => spriteName(index));
}
