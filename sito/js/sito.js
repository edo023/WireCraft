// WireCraft: piccoli effetti del sito. Senza JavaScript tutto il contenuto resta visibile.
(function () {
  "use strict";
  var doc = document.documentElement;
  doc.classList.add("js");
  var calmo = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  document.addEventListener("DOMContentLoaded", function () {
    // menu su schermi piccoli
    var bottone = document.querySelector(".apri-menu");
    var menu = document.getElementById("menu");
    if (bottone && menu) {
      bottone.addEventListener("click", function () {
        var aperto = menu.classList.toggle("aperto");
        bottone.setAttribute("aria-expanded", aperto ? "true" : "false");
      });
      menu.addEventListener("click", function (e) {
        if (e.target.tagName === "A") {
          menu.classList.remove("aperto");
          bottone.setAttribute("aria-expanded", "false");
        }
      });
    }

    // comparsa delle sezioni allo scorrimento
    var daRivelare = document.querySelectorAll(".rivela");
    if (!("IntersectionObserver" in window) || calmo) {
      daRivelare.forEach(function (el) { el.classList.add("vista"); });
    } else {
      var oss = new IntersectionObserver(function (voci) {
        voci.forEach(function (v) {
          if (v.isIntersecting) { v.target.classList.add("vista"); oss.unobserve(v.target); }
        });
      }, { rootMargin: "0px 0px -8% 0px", threshold: 0.08 });
      daRivelare.forEach(function (el) { oss.observe(el); });
    }

    // il terminale: le righe arrivano una dopo l'altra, come una conversazione
    var corpo = document.querySelector(".terminale-corpo");
    if (corpo) {
      var righe = corpo.querySelectorAll(".riga");
      var parti = function () {
        var ritardi = [0, 900, 1500, 1750, 2000, 2250, 2500, 2750, 3000, 3700];
        righe.forEach(function (r, i) {
          setTimeout(function () {
            righe.forEach(function (x) { x.classList.remove("cursore"); });
            r.classList.add("vista");
            if (i === righe.length - 1) r.classList.add("cursore");
          }, calmo ? 0 : (ritardi[i] !== undefined ? ritardi[i] : 3000 + i * 250));
        });
      };
      if (!("IntersectionObserver" in window) || calmo) {
        parti();
      } else {
        var ossT = new IntersectionObserver(function (voci) {
          if (voci[0].isIntersecting) { parti(); ossT.disconnect(); }
        }, { threshold: 0.35 });
        ossT.observe(corpo);
      }
    }

    // un alone di luce che segue il puntatore sulle schede delle funzioni
    if (!calmo) {
      document.querySelectorAll(".funzione").forEach(function (s) {
        s.addEventListener("pointermove", function (e) {
          var r = s.getBoundingClientRect();
          s.style.setProperty("--mx", (e.clientX - r.left) + "px");
          s.style.setProperty("--my", (e.clientY - r.top) + "px");
        });
      });
    }
  });
})();
