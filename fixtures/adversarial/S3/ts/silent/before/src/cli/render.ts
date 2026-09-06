export function render(rows: string[]): void {
  process.stdout.write(rows.join("\n"));
}
