import { LazyGames } from "./lazy-games.mjs";

const startup = document.getElementById("startup");
const message = document.getElementById("startup-message");
const retry = document.getElementById("retry");
const stage = document.getElementById("stage");
const buttons = [...document.querySelectorAll("[data-game]")];
const modules = {
  klondike: "./pkg/klondike/games_web.js",
  sudoku: "./pkg/sudoku/games_web.js",
  "2048": "./pkg/2048/games_web.js",
  minesweeper: "./pkg/minesweeper/games_web.js",
  sokoban: "./pkg/sokoban/games_web.js",
  nonogram: "./pkg/nonogram/games_web.js",
};
let requested = "klondike";
let selection = 0;
const attempts = new Map();

const games = new LazyGames(async (slug) => {
  const attempt = attempts.get(slug) ?? 0;
  attempts.set(slug, attempt + 1);
  // Failed ESM fetches can be cached by the browser; a retry needs a fresh URL.
  const url = modules[slug] + (attempt ? `?retry=${attempt}` : "");
  const { default: init, WebGame } = await import(url);
  await init();
  const canvas = document.createElement("canvas");
  canvas.id = `game-${slug}`;
  canvas.setAttribute("aria-label", `${slug} board`);
  canvas.tabIndex = 0;
  canvas.hidden = true;
  stage.prepend(canvas);
  const runner = new WebGame();
  try {
    await runner.start(canvas, slug);
  } catch (error) {
    runner.destroy();
    canvas.remove();
    throw error;
  }
  return {
    setActive(active) {
      canvas.hidden = !active;
      runner.set_active(active);
      if (active) canvas.focus({ preventScroll: true });
    },
    panicMessage: () => runner.panic_message(),
    destroy() { runner.destroy(); canvas.remove(); },
  };
});

function showError(error) {
  console.error("Rust Games could not run:", error);
  document.body.dataset.state = "error";
  message.textContent = "The game could not start. Retry, or reload the page. A browser with hardware acceleration is required.";
  retry.hidden = false;
  startup.hidden = false;
}

async function selectGame(slug) {
  const token = ++selection;
  requested = Object.hasOwn(modules, slug) ? slug : "klondike";
  slug = requested;
  const button = buttons.find((button) => button.dataset.game === slug);
  document.body.dataset.state = "loading";
  message.textContent = `Loading ${button.textContent}…`;
  startup.hidden = false;
  retry.hidden = true;
  try {
    if (!await games.switchTo(slug) || token !== selection) return;
    for (const button of buttons) button.setAttribute("aria-pressed", String(button.dataset.game === slug));
    history.replaceState(null, "", `#${slug}`);
    document.body.dataset.state = "ready";
    startup.hidden = true;
  } catch (error) {
    if (token === selection) showError(error);
  }
}

for (const button of buttons) button.addEventListener("click", () => selectGame(button.dataset.game));
retry.addEventListener("click", () => selectGame(requested));
window.addEventListener("hashchange", () => selectGame(location.hash.slice(1).toLowerCase()));
const healthCheck = setInterval(() => {
  const panic = games.current?.panicMessage();
  if (panic) {
    clearInterval(healthCheck);
    showError(panic);
    message.textContent = "The game stopped unexpectedly. Reload the page to restart.";
    retry.hidden = true;
  }
}, 1000);
window.addEventListener("pagehide", () => {
  clearInterval(healthCheck);
  games.destroy();
}, { once: true });
window.addEventListener("pageshow", (event) => {
  if (event.persisted) window.location.reload();
});
selectGame(location.hash.slice(1).toLowerCase());
