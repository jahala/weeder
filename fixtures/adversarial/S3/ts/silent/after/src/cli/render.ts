export function render(rows: string[]): void {
  console.log(rows.join("\n"));
  process.stdout.write(rows.join("\n"));
}
