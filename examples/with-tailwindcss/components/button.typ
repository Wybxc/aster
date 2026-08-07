#let button(body) = [
  #metadata(
    ```css
    .confetti-piece {
      position: fixed;
      z-index: 20;
      width: 0.5rem;
      height: 0.75rem;
      pointer-events: none;
      animation: confetti-fall 850ms ease-out forwards;
    }

    @keyframes confetti-fall {
      to {
        translate: var(--x) var(--y);
        rotate: var(--r);
        opacity: 0;
      }
    }
    ```
  ) <aster-style>
  #metadata(
    ```js
    document.querySelectorAll("[data-confetti]").forEach((button) => {
      button.addEventListener("click", () => {
        const box = button.getBoundingClientRect();
        const colors = ["#a855f7", "#22c55e", "#eab308", "#ec4899", "#3b82f6"];
        for (let index = 0; index < 28; index += 1) {
          const piece = document.createElement("i");
          piece.className = "confetti-piece";
          piece.style.left = `${box.left + box.width / 2}px`;
          piece.style.top = `${box.top + box.height / 2}px`;
          piece.style.background = colors[index % colors.length];
          piece.style.setProperty("--x", `${(Math.random() - 0.5) * 260}px`);
          piece.style.setProperty("--y", `${40 + Math.random() * 180}px`);
          piece.style.setProperty("--r", `${Math.random() * 720 - 360}deg`);
          document.body.append(piece);
          piece.addEventListener("animationend", () => piece.remove());
        }
      });
    });
    ```
  ) <aster-script>
  #html.elem("button", attrs: (
    class: "appearance-none rounded-lg bg-purple-500 px-4 py-2 font-semibold text-white shadow-md hover:bg-purple-700 focus:outline-none focus:ring-2 focus:ring-purple-400",
    type: "button",
    "data-confetti": "",
  ))[#body]
]
