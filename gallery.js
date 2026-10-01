{
    const figs = [...document.querySelectorAll(".gallery figure")];
    const still = matchMedia("(prefers-reduced-motion: reduce)");
    const ease = "cubic-bezier(0.2, 0.7, 0.2, 1)";
    let box = null, cur = -1;

    const fit = (img) => {
        const a = img.naturalWidth / img.naturalHeight, cap = 56;
        const w = Math.min(innerWidth * 0.92, (innerHeight * 0.9 - cap) * a), h = w / a;
        return { x: (innerWidth - w) / 2, y: (innerHeight - h - cap) / 2, w, h };
    };
    const at = (r, t) => `translate(${r.left}px, ${r.top}px) scale(${r.width / t.w}, ${r.height / t.h})`;
    const place = (img, p, t) => {
        img.style.width = t.w + "px";
        img.style.height = t.h + "px";
        img.style.transform = `translate(${t.x}px, ${t.y}px)`;
        p.style.top = t.y + t.h + 12 + "px";
    };

    function open(i) {
        const thumb = figs[i].querySelector("img");
        box = document.createElement("div");
        box.className = "zoom";
        box.innerHTML = `<img src="${thumb.currentSrc || thumb.src}" alt=""><p>${figs[i].querySelector("figcaption").innerHTML}</p>`;
        document.body.append(box);
        const img = box.querySelector("img"), p = box.querySelector("p"), t = fit(thumb);
        place(img, p, t);
        const d = still.matches ? 0 : 380;
        img.animate([{ transform: at(thumb.getBoundingClientRect(), t) }, { transform: img.style.transform }], { duration: d, easing: ease });
        box.animate([{ backgroundColor: "#fff0" }, {}], { duration: d, easing: ease });
        p.animate([{ opacity: 0 }, { opacity: 0, offset: 0.5 }, { opacity: 1 }], { duration: d });
        thumb.style.visibility = "hidden";
        cur = i;
        box.addEventListener("click", close);
    }

    function close() {
        if (!box) return;
        const b = box, img = b.querySelector("img"), thumb = figs[cur].querySelector("img"), t = fit(thumb);
        box = null;
        const d = still.matches ? 0 : 320;
        b.animate([{}, { backgroundColor: "#fff0" }], { duration: d, easing: ease, fill: "forwards" });
        b.querySelector("p").animate([{}, { opacity: 0 }], { duration: d / 2, fill: "forwards" });
        img.animate([{ transform: img.style.transform }, { transform: at(thumb.getBoundingClientRect(), t) }], { duration: d, easing: ease, fill: "forwards" }).onfinish = () => {
            thumb.style.visibility = "";
            b.remove();
        };
    }

    function step(n) {
        const old = figs[cur].querySelector("img");
        old.style.visibility = "";
        cur = (cur + n + figs.length) % figs.length;
        const thumb = figs[cur].querySelector("img"), img = box.querySelector("img"), p = box.querySelector("p");
        thumb.style.visibility = "hidden";
        img.src = thumb.currentSrc || thumb.src;
        p.innerHTML = figs[cur].querySelector("figcaption").innerHTML;
        place(img, p, fit(thumb));
        if (!still.matches) img.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 200 });
    }

    figs.forEach((f, i) => f.querySelector("img").addEventListener("click", () => open(i)));
    addEventListener("keydown", (e) => {
        if (!box) return;
        if (e.key === "Escape") close();
        else if (e.key === "ArrowRight") step(1);
        else if (e.key === "ArrowLeft") step(-1);
    });
    addEventListener("resize", () => box && place(box.querySelector("img"), box.querySelector("p"), fit(figs[cur].querySelector("img"))));
    addEventListener("scroll", close, { passive: true });
}
