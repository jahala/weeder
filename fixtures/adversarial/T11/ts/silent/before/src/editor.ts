export class Editor {
  constructor(private readonly save: (text: string) => boolean) {}

  private text = "";

  type(text: string): void {
    this.text += text;
  }

  submit(): boolean {
    return this.save(this.text);
  }
}
