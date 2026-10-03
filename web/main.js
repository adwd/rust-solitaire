const startup = document.getElementById("startup");
const message = document.getElementById("startup-message");

function showError(error) {
  console.error("Rust Solitaire could not run:", error);
  document.body.dataset.state = "error";
  message.textContent = "The game could not start. Reload the page, or try an up-to-date browser with hardware acceleration enabled.";
  startup.hidden = false;
}

try {
  const { default: init, WebGame } = await import("./pkg/solitaire_web.js");
  await init();
  const game = new WebGame();
  await game.start(document.getElementById("game"));
  document.body.dataset.state = "ready";
  startup.hidden = true;
  const healthCheck = setInterval(() => {
    const panic = game.panic_message();
    if (panic) {
      clearInterval(healthCheck);
      showError(panic);
    }
  }, 1000);
  window.addEventListener("pagehide", () => {
    clearInterval(healthCheck);
    game.destroy();
  }, { once: true });
  window.addEventListener("pageshow", (event) => {
    if (event.persisted) window.location.reload();
  });
} catch (error) {
  showError(error);
}
