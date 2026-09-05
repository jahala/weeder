import { render } from "../core/finding";
import { read } from "../seams/git";

export function check(path: string): string {
  return render({ rule: "T1", path: read(path) });
}
