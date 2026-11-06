export function trim(value: string): string {
  return value.trim();
}

export function slugify(value: string): string {
  return trim(value).toLowerCase().split(" ").join("-");
}
