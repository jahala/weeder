import { Strategy } from "./client";

export class Fast extends Strategy {
  send(payload: string): string {
    return payload.trim();
  }

  retry(payload: string): string {
    return this.send(payload);
  }
}
