export const SERVICE = "build.example.app";

export function endpoint(host: string): string {
  return `https://${host}/`;
}
