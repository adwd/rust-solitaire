// Each module loads once. A slow earlier request must not replace the latest
// selection. Retaining the runtime keeps the full game and undo history alive.
export class LazyGames {
  constructor(load) {
    this.load = load;
    this.pending = new Map();
    this.loaded = new Map();
    this.current = null;
    this.request = 0;
    this.closed = false;
  }

  async switchTo(slug) {
    if (this.closed) return false;
    const request = ++this.request;
    if (!this.pending.has(slug)) {
      const promise = Promise.resolve().then(() => this.load(slug)).then((game) => {
        if (this.closed) game.destroy();
        else this.loaded.set(slug, game);
        return game;
      }).catch((error) => {
        this.pending.delete(slug);
        throw error;
      });
      this.pending.set(slug, promise);
    }
    let game;
    try { game = await this.pending.get(slug); }
    catch (error) {
      if (request !== this.request || this.closed) return false;
      throw error;
    }
    if (request !== this.request || this.closed) return false;
    if (this.current !== game) {
      this.current?.setActive(false);
    }
    game.setActive(true);
    this.current = game;
    return true;
  }

  destroy() {
    this.closed = true;
    ++this.request;
    for (const game of this.loaded.values()) game.destroy();
    this.loaded.clear();
    this.pending.clear();
    this.current = null;
  }
}
