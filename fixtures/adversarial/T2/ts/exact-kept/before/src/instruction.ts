export function instruction(branch: string, base: string, remote: string): string {
  return `Rebase ${branch} onto ${base} from ${remote}.`;
}
