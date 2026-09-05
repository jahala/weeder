export function send(payload: string): string {
  return payload.trim();
}

export function label(kind: string): string {
  return kind === "todo" ? "TODO: written by the caller" : "done";
}

export function version(): string {
  return "1.4.0";
}
