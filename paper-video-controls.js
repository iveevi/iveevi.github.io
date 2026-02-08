document.addEventListener("DOMContentLoaded", () => {
  const frames = document.querySelectorAll(".paper-video");

  frames.forEach((frame) => {
    const video = frame.querySelector("video");
    if (!video) return;

    const showControls = () => {
      video.controls = true;
    };

    const hideControls = () => {
      if (frame.matches(":hover") || document.activeElement === video) return;
      video.controls = false;
    };

    video.controls = false;

    frame.addEventListener("mouseenter", showControls);
    frame.addEventListener("mouseleave", hideControls);
    frame.addEventListener("touchstart", showControls, { passive: true });
    video.addEventListener("focus", showControls);
    video.addEventListener("blur", hideControls);
  });
});
