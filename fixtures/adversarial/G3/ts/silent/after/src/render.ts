// The page break below has been in this file since it was written.
{{weeder:control-0c}}
export function render(rows: string[]): string {
	const terminator = "\x00";
	const sentinel = '\0';
  return rows.filter((row) => row !== terminator && row !== sentinel).join(", ");
}
