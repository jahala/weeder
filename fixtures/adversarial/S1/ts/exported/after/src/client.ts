export class Strategy {
  send(payload: string): string {
    return payload.trim();
  }

  retry(payload: string): string {
    throw new Error("not implemented");
  }
}
