for (const v of document.querySelectorAll("video.ffwd")) {
    const again = () => {
        v.currentTime = 0;
        v.play().catch(() => {});
    };
    v.addEventListener("ended", again);
    v.addEventListener("pause", () => {
        if (v.duration && v.currentTime >= v.duration - 0.1) again();
    });
}
