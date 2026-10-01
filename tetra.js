{
    const cv = document.getElementById("tetra");
    const AX = [
        { name: "Neural graphics", c: [226, 59, 64] },
        { name: "Real-time rendering", c: [40, 160, 70] },
        { name: "Programming systems", c: [40, 100, 225] },
        { name: "Appearance modeling", c: [235, 180, 20] },
    ];
    const PAPERS = [
        { name: "RCGP", title: "RCGP: Resource Contracts for Graphics Programming", href: "rcgp/", w: [0, 0.25, 0.75, 0] },
        { name: "Glow", title: "Modeling and Rendering Glow Discharge", href: "glow/", w: [0, 0.3, 0, 0.7] },
        { name: "GFS", title: "Geometry Field Splatting with Gaussian Surfels", href: "https://arxiv.org/abs/2411.17067", w: [0.6, 0.2, 0, 0.2] },
        { name: "NGF", title: "Neural Geometry Fields for Meshes", href: "ngf/", w: [0.75, 0.25, 0, 0] },
        { name: "ReSTIR DR", title: "Parameter-space ReSTIR for Differentiable and Inverse Rendering", href: "https://weschang.com/publications/restir-dr/", w: [0, 0.5, 0, 0.5] },
    ];
    const r = Math.sqrt(8) / 3;
    const V = [[0, 1, 0], ...[90, 210, 330].map((d) => [r * Math.cos((d * Math.PI) / 180), -1 / 3, r * Math.sin((d * Math.PI) / 180)])];
    const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    const F = [[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]].map((f) => {
        const n = cross(sub(V[f[1]], V[f[0]]), sub(V[f[2]], V[f[0]]));
        const m = [0, 1, 2].map((k) => (V[f[0]][k] + V[f[1]][k] + V[f[2]][k]) / 3);
        return dot(n, m) < 0 ? { v: [f[0], f[2], f[1]], n: n.map((x) => -x) } : { v: f, n };
    });
    const E = [[0, 1], [0, 2], [0, 3], [1, 2], [2, 3], [3, 1]];
    const pos = PAPERS.map((p) => {
        const t = p.w.reduce((a, b) => a + b, 0);
        const w = p.w.map((x) => x / t);
        return {
            ...p,
            p: [0, 1, 2].map((k) => w.reduce((s, x, i) => s + x * V[i][k], 0)),
            c: [0, 1, 2].map((k) => Math.round(w.reduce((s, x, i) => s + x * AX[i].c[k], 0))),
        };
    });
    const still = matchMedia("(prefers-reduced-motion: reduce)");
    const off = document.createElement("canvas"), octx = off.getContext("2d");
    const ctx = cv.getContext("2d");
    let th = 0.5, ph = 0.35, W = 0, H = 0, S = 0, dpr = 1, live = false, drag = null, hover = -1, hoverT = 0, last = 0, shown = [];

    const rot = (p) => {
        const c = Math.cos(th), s = Math.sin(th), cp = Math.cos(ph), sp = Math.sin(ph);
        const x = p[0] * c + p[2] * s, z = -p[0] * s + p[2] * c;
        return [x, p[1] * cp - z * sp, p[1] * sp + z * cp];
    };
    const scr = (q) => [W / 2 + q[0] * S, H / 2 - q[1] * S, q[2]];

    function size() {
        dpr = Math.min(2, devicePixelRatio || 1);
        W = cv.clientWidth;
        const B = Math.min(W, 432);
        H = Math.round(B * 0.5 + 72);
        cv.style.height = H + "px";
        cv.width = W * dpr;
        cv.height = H * dpr;
        off.width = W;
        off.height = H;
        S = B * 0.25;
    }

    function faces(P, front) {
        const img = octx.createImageData(W, H), d = img.data;
        const order = F.map((f, i) => ({ f, i, z: (P[f.v[0]][2] + P[f.v[1]][2] + P[f.v[2]][2]) / 3 })).sort((a, b) => a.z - b.z);
        for (const { f, i } of order) {
            const [a, b, c] = f.v.map((k) => P[k]);
            const A = front[i] ? 0.24 : 0.12;
            const den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
            const x0 = Math.max(0, Math.floor(Math.min(a[0], b[0], c[0]))), x1 = Math.min(W - 1, Math.ceil(Math.max(a[0], b[0], c[0])));
            const y0 = Math.max(0, Math.floor(Math.min(a[1], b[1], c[1]))), y1 = Math.min(H - 1, Math.ceil(Math.max(a[1], b[1], c[1])));
            const ca = AX[f.v[0]].c, cb = AX[f.v[1]].c, cc = AX[f.v[2]].c;
            for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) {
                const px = x + 0.5, py = y + 0.5;
                const l0 = ((b[1] - c[1]) * (px - c[0]) + (c[0] - b[0]) * (py - c[1])) / den;
                const l1 = ((c[1] - a[1]) * (px - c[0]) + (a[0] - c[0]) * (py - c[1])) / den;
                const l2 = 1 - l0 - l1;
                if (l0 < 0 || l1 < 0 || l2 < 0) continue;
                const o = (y * W + x) * 4, da = d[o + 3] / 255, na = A + da * (1 - A);
                for (let k = 0; k < 3; k++) d[o + k] = ((l0 * ca[k] + l1 * cb[k] + l2 * cc[k]) * A + d[o + k] * da * (1 - A)) / na;
                d[o + 3] = na * 255;
            }
        }
        octx.putImageData(img, 0, 0);
    }

    function draw() {
        const P = V.map((v) => scr(rot(v)));
        const front = F.map((f) => rot(f.n)[2] > 0);
        faces(P, front);
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, W, H);
        ctx.drawImage(off, 0, 0, W, H);
        for (const [i, j] of E) {
            const vis = F.some((f, k) => front[k] && f.v.includes(i) && f.v.includes(j));
            ctx.beginPath();
            ctx.moveTo(P[i][0], P[i][1]);
            ctx.lineTo(P[j][0], P[j][1]);
            ctx.setLineDash(vis ? [] : [3, 4]);
            ctx.strokeStyle = vis ? "rgba(27,27,31,0.55)" : "rgba(27,27,31,0.3)";
            ctx.lineWidth = 1.2;
            ctx.stroke();
        }
        ctx.setLineDash([]);
        ctx.font = "700 13px 'Fira Sans Condensed', sans-serif";
        ctx.textBaseline = "middle";
        ctx.globalAlpha = hover >= 0 ? 0.3 : 1;
        AX.forEach((a, i) => {
            const dx = P[i][0] - W / 2, dy = P[i][1] - H / 2, l = Math.hypot(dx, dy) || 1;
            const x = P[i][0] + (dx / l) * 16, y = P[i][1] + (dy / l) * 14;
            ctx.textAlign = Math.abs(dx / l) < 0.35 ? "center" : dx > 0 ? "left" : "right";
            ctx.fillStyle = `rgb(${a.c})`;
            ctx.beginPath();
            ctx.arc(P[i][0], P[i][1], 3.5, 0, 7);
            ctx.fill();
            ctx.lineWidth = 4;
            ctx.strokeStyle = "rgba(255,255,255,0.9)";
            ctx.lineJoin = "round";
            const lines = a.name.split(" "), y0 = y - ((lines.length - 1) * 14) / 2 + (dy > l * 0.35 ? 7 : dy < -l * 0.35 ? -7 : 0);
            lines.forEach((ln, k) => ctx.strokeText(ln, x, y0 + k * 14));
            ctx.fillStyle = "#1b1b1f";
            lines.forEach((ln, k) => ctx.fillText(ln, x, y0 + k * 14));
        });
        ctx.globalAlpha = 1;
        shown = pos.map((p, i) => ({ i, q: scr(rot(p.p)) })).sort((a, b) => a.q[2] - b.q[2]);
        for (const { i, q } of shown) {
            const p = pos[i], R = hover === i ? 7.5 : 6;
            ctx.globalAlpha = 0.55 + 0.45 * Math.min(1, Math.max(0, (q[2] + 1) / 2));
            ctx.beginPath();
            ctx.arc(q[0], q[1], R, 0, 7);
            ctx.fillStyle = `rgb(${p.c})`;
            ctx.fill();
            ctx.lineWidth = 2;
            ctx.strokeStyle = "#fff";
            ctx.stroke();
        }
        ctx.globalAlpha = 1;
        if (hover >= 0) {
            const p = pos[hover], q = shown.find((x) => x.i === hover).q;
            let fs = 14;
            const font = () => (ctx.font = `700 ${fs}px 'Fira Sans Condensed', sans-serif`);
            font();
            let tw = ctx.measureText(p.title).width;
            const d = 16, R0 = 8, gap = R0 * 0.7 + d + 6;
            const roomR = W - q[0] - gap, roomL = q[0] - gap;
            let right = tw <= roomR && (tw > roomL || roomR >= roomL) ? true : tw <= roomL ? false : roomR >= roomL;
            const room = right ? roomR : roomL;
            let lines = [p.title];
            if (tw > room) {
                fs = Math.max(12, Math.floor((fs * room) / tw));
                font();
                tw = ctx.measureText(p.title).width;
                if (tw > room) {
                    const words = p.title.split(" ");
                    let best = null;
                    for (let i = 1; i < words.length; i++) {
                        const ls = [words.slice(0, i).join(" "), words.slice(i).join(" ")], w = Math.max(...ls.map((l) => ctx.measureText(l).width));
                        if (!best || w < best.w) best = { ls, w };
                    }
                    lines = best.ls;
                    tw = best.w;
                }
            }
            const k = still.matches ? 1 : Math.max(0, Math.min(1, (performance.now() - hoverT) / 420)), e = 1 - (1 - k) ** 3;
            const lh = fs * 1.2, th2 = lh * lines.length;
            const up = q[1] - d - th2 - 6 > 0, sx = right ? 1 : -1, sy = up ? -1 : 1;
            const x0 = q[0] + sx * R0 * 0.7, y0 = q[1] + sy * R0 * 0.7, x1 = x0 + sx * d, y1 = y0 + sy * d;
            const L = tw + 4, a1 = Math.min(1, e / 0.3), a2 = Math.max(0, (e - 0.3) / 0.7);
            ctx.beginPath();
            ctx.moveTo(x0, y0);
            ctx.lineTo(x0 + sx * d * a1, y0 + sy * d * a1);
            if (a2 > 0) ctx.lineTo(x1 + sx * L * a2, y1);
            ctx.lineWidth = 3.5;
            ctx.lineJoin = "round";
            ctx.strokeStyle = "rgba(255,255,255,0.85)";
            ctx.stroke();
            ctx.lineWidth = 1.2;
            ctx.strokeStyle = "#1b1b1f";
            ctx.stroke();
            if (a2 > 0) {
                ctx.save();
                ctx.beginPath();
                ctx.rect(right ? x1 : x1 - L * a2, y1 - th2 - 6, L * a2, th2 + 6);
                ctx.clip();
                ctx.textAlign = right ? "left" : "right";
                ctx.textBaseline = "alphabetic";
                const tx = x1 + sx * 2, ty = y1 - 4;
                ctx.lineWidth = 4;
                ctx.strokeStyle = "rgba(255,255,255,0.85)";
                lines.forEach((ln, i) => ctx.strokeText(ln, tx, ty - (lines.length - 1 - i) * lh));
                ctx.fillStyle = "#1b1b1f";
                lines.forEach((ln, i) => ctx.fillText(ln, tx, ty - (lines.length - 1 - i) * lh));
                ctx.restore();
            }
        }
    }

    function tick(t) {
        if (!live) return;
        const dt = last ? Math.min(0.05, (t - last) / 1000) : 0;
        last = t;
        if (!drag && !still.matches && hover < 0) th += dt * 0.25;
        draw();
        requestAnimationFrame(tick);
    }

    const hit = (e) => {
        const b = cv.getBoundingClientRect(), x = e.clientX - b.left, y = e.clientY - b.top;
        for (let k = shown.length - 1; k >= 0; k--) if (Math.hypot(shown[k].q[0] - x, shown[k].q[1] - y) < 11) return shown[k].i;
        return -1;
    };
    cv.addEventListener("pointerdown", (e) => {
        drag = { x: e.clientX, y: e.clientY, th, ph, moved: false };
        cv.setPointerCapture(e.pointerId);
    });
    cv.addEventListener("pointermove", (e) => {
        if (drag) {
            const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
            if (Math.hypot(dx, dy) > 3) drag.moved = true;
            th = drag.th + dx * 0.012;
            ph = Math.max(-1.2, Math.min(1.2, drag.ph + dy * 0.012));
            if (still.matches) draw();
            return;
        }
        const h = hit(e);
        if (h !== hover) {
            hover = h;
            hoverT = performance.now();
            cv.style.cursor = h < 0 ? "grab" : "pointer";
            if (still.matches) draw();
        }
    });
    cv.addEventListener("pointerup", (e) => {
        const moved = drag && drag.moved;
        drag = null;
        if (!moved) {
            const h = hit(e);
            if (h >= 0) location.href = pos[h].href;
        }
    });
    cv.addEventListener("pointerleave", () => {
        hover = -1;
    });
    new IntersectionObserver(([e]) => {
        const was = live;
        live = e.isIntersecting && cv.clientWidth > 0;
        if (live && !was) {
            last = 0;
            requestAnimationFrame(tick);
        }
    }).observe(cv);
    addEventListener("resize", () => {
        size();
        draw();
    });
    size();
    document.fonts.ready.then(() => {
        size();
        draw();
    });
}
