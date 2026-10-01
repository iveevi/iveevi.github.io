for (const a of document.querySelectorAll(".copy-bib")) a.addEventListener("click", async (e) => {
    e.preventDefault();
    const text = document.getElementById("bibtex").textContent;
    try {
        await navigator.clipboard.writeText(text);
    } catch {
        const t = document.createElement("textarea");
        t.value = text;
        document.body.append(t);
        t.select();
        document.execCommand("copy");
        t.remove();
    }
    a.classList.add("copied");
    clearTimeout(a.timer);
    a.timer = setTimeout(() => a.classList.remove("copied"), 1500);
});
