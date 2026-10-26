export function spawnPane(name: string): string {
  return `tmux new-window -n ${name}`;
}
