document.body.classList.add("js");

// copy-to-clipboard for command chips
document.querySelectorAll(".cmd[data-copy]").forEach((btn) => {
  btn.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(btn.dataset.copy);
      if (!btn.querySelector(".copied")) {
        const ok = document.createElement("span");
        ok.className = "copied";
        ok.textContent = "kopiert ✓";
        btn.appendChild(ok);
        setTimeout(() => ok.remove(), 1600);
      }
    } catch (e) {
      /* clipboard unavailable — leave the text selectable */
    }
  });
});

// easter egg: the hero shows a fresh password on every visit, and the
// "Kopiert ✓" chip really copies it. Same alphabet and length as the
// app's defaults: all four character sets, three groups of five.
const POOL =
  "abcdefghijklmnopqrstuvwxyz" +
  "ABCDEFGHIJKLMNOPQRSTUVWXYZ" +
  "0123456789" +
  "!@#$%^&*";

function randomIndex(n) {
  // rejection sampling, so every character is equally likely
  const limit = 256 - (256 % n);
  const buf = new Uint8Array(1);
  do {
    crypto.getRandomValues(buf);
  } while (buf[0] >= limit);
  return buf[0] % n;
}

function generatePassword(groups) {
  const parts = [];
  for (let g = 0; g < groups; g++) {
    let part = "";
    for (let i = 0; i < 5; i++) part += POOL[randomIndex(POOL.length)];
    parts.push(part);
  }
  return parts.join("-");
}

const typed = document.querySelector(".typed");
const egg = document.querySelector(".chip.egg");
if (typed && egg && window.crypto && crypto.getRandomValues) {
  typed.textContent = generatePassword(3);
  egg.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(typed.textContent);
      egg.textContent = "Echt kopiert ✓";
      setTimeout(() => (egg.textContent = "Kopiert ✓"), 1600);
    } catch (e) {
      /* clipboard unavailable */
    }
  });
}

// gentle reveal on scroll
const io = new IntersectionObserver(
  (entries) => {
    entries.forEach((en) => {
      if (en.isIntersecting) {
        en.target.classList.add("in");
        io.unobserve(en.target);
      }
    });
  },
  { rootMargin: "0px 0px -8% 0px" }
);
document.querySelectorAll(".reveal").forEach((el) => io.observe(el));
