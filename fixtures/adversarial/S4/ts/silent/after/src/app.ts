import { slugify, trim } from "./text";

export function title(input: string): string {
  return trim(input);
}

export function link(input: string): string {
  return "/posts/" + slugify(input);
}
