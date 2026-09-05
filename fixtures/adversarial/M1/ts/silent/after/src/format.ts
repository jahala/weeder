export function format(value: string, width: number): string {
  if (value.length > width) {
    return value.slice(0, width);
  }
  return value.padEnd(width);
}
