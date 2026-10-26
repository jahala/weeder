export const STOP_HOOK_SCRIPT = "#!/bin/sh\nexit 0\n";

export function writeHooks(dir: string): string {
  return `${dir}/stop.sh`;
}
