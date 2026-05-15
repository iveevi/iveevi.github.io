// Click-to-copy BibTeX pill.
// Reads the citation from a <script type="text/plain"> element referenced
// by the button's data-bibtex-source attribute and writes it to the clipboard.

(function () {
  function copyText(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      return navigator.clipboard.writeText(text);
    }
    // Fallback for older browsers / insecure contexts.
    return new Promise((resolve, reject) => {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.setAttribute('readonly', '');
      ta.style.position = 'fixed';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.select();
      try {
        document.execCommand('copy');
        resolve();
      } catch (err) {
        reject(err);
      } finally {
        document.body.removeChild(ta);
      }
    });
  }

  function flash(btn, msg, ms) {
    if (!btn._originalHTML) {
      btn._originalHTML = btn.innerHTML;
    }
    btn.innerHTML = msg;
    btn.classList.add('is-copied');
    if (btn._flashTimer) clearTimeout(btn._flashTimer);
    btn._flashTimer = setTimeout(() => {
      btn.innerHTML = btn._originalHTML;
      btn.classList.remove('is-copied');
    }, ms);
  }

  function init() {
    document.querySelectorAll('.copy-bibtex').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.preventDefault();
        const sel = btn.getAttribute('data-bibtex-source');
        if (!sel) return;
        const src = document.querySelector(sel);
        if (!src) return;
        const text = src.textContent.trim();
        copyText(text).then(
          () => flash(btn, 'Copied!', 1600),
          () => flash(btn, 'Copy failed', 1600)
        );
      });
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
