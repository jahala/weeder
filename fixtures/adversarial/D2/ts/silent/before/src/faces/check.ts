import { render } from "../core/finding";

export function check(path: string): string {
  return render({ rule: "T1", path });
}
