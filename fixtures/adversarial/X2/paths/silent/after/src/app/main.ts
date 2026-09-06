import { sendPayload } from "../wire/client";

export function run(rows: string[]): string {
  return sendPayload(rows.join(","));
}
