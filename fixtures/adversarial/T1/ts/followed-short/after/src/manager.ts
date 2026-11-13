/** The connection manager keeps one connection open. */
export class Manager {
  open(dial: () => boolean): boolean {
    return dial();
  }
}
