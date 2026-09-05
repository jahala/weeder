export function summarise(rows: string[]): string {
  console.log("rows", rows);
  return rows.join(", ");
}

export function total(rows: number[]): number {
  debugger;
  return rows.reduce((sum, row) => sum + row, 0);
}
