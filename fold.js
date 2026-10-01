const folds = document.querySelectorAll(".fold > h2 > button");
const fit = () => {
    for (const b of folds) b.parentElement.style.setProperty("--tw", b.offsetWidth + "px");
};
for (const b of folds) b.addEventListener("click", () => {
    const open = b.closest(".fold").classList.toggle("open");
    b.setAttribute("aria-expanded", String(open));
});
fit();
document.fonts.ready.then(fit);
addEventListener("resize", fit);
for (const b of document.querySelectorAll(".more[aria-controls]")) b.addEventListener("click", () => {
    const open = document.getElementById(b.getAttribute("aria-controls")).classList.toggle("open");
    b.setAttribute("aria-expanded", String(open));
});
