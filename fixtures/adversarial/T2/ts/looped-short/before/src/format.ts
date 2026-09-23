export function format(value: string, width: number): string {
  return value.length > width ? value.slice(0, width) : value.padEnd(width);
}
