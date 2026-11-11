export function banner(error: Error): string {
  return `<p role="alert">${error.message}</p>`;
}
