export function send(payload: string): string {
  return payload.trim();
}

export class Strategy {
  send(payload: string): string {
    return send(payload);
  }
}
