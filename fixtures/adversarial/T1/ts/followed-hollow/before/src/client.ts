/** The client talks to the server. */
export class Client {
  connect(dial: () => boolean): boolean {
    return dial();
  }
}
