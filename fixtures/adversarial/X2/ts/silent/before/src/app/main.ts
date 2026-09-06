import { sendPayload } from "../wire/client";
import { summarise } from "../report/summary";

export function run(rows: string[]): string {
  const sent = sendPayload(rows.join(","));
  return summarise([sent]);
}
