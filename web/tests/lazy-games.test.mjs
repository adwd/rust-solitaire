import { test } from "node:test";
import assert from "node:assert/strict";
import { LazyGames } from "../lazy-games.mjs";

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function game() {
  return { active: false, moves: 0, destroyed: false,
    setActive(value) { this.active = value; }, destroy() { this.destroyed = true; } };
}

test("loads only selected games and retains progress across switches", async () => {
  const loads = [];
  const games = new LazyGames(async (slug) => { loads.push(slug); return game(); });
  assert.deepEqual(loads, []);
  await games.switchTo("sokoban");
  const sokoban = games.current;
  sokoban.moves = 12;
  await games.switchTo("nonogram");
  assert.equal(sokoban.active, false);
  await games.switchTo("sokoban");
  assert.equal(games.current, sokoban);
  assert.equal(games.current.moves, 12);
  assert.deepEqual(loads, ["sokoban", "nonogram"]);
});

test("rapid switches keep the newest choice when earlier loading is slower", async () => {
  const a = deferred(), b = deferred();
  const games = new LazyGames((slug) => slug === "a" ? a.promise : b.promise);
  const first = games.switchTo("a"), second = games.switchTo("b");
  const latest = game();
  b.resolve(latest);
  assert.equal(await second, true);
  const stale = game();
  a.resolve(stale);
  assert.equal(await first, false);
  assert.equal(games.current, latest);
  assert.equal(stale.active, false);
});

test("failed downloads can retry without losing the active game", async () => {
  let attempt = 0;
  const games = new LazyGames(async (slug) => {
    if (slug === "b" && attempt++ === 0) throw new Error("offline");
    return game();
  });
  await games.switchTo("a");
  const previous = games.current;
  await assert.rejects(games.switchTo("b"), /offline/);
  assert.equal(previous.active, true);
  await games.switchTo("b");
  assert.equal(previous.active, false);
});

test("pending requests share one download and teardown destroys late loads", async () => {
  const pending = deferred();
  let loads = 0;
  const games = new LazyGames(() => { ++loads; return pending.promise; });
  const a = games.switchTo("a"), b = games.switchTo("a");
  await Promise.resolve();
  games.destroy();
  const loaded = game();
  pending.resolve(loaded);
  assert.deepEqual(await Promise.all([a, b]), [false, false]);
  assert.equal(loads, 1);
  assert.equal(loaded.destroyed, true);
});
