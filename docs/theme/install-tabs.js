(function () {
  var STORAGE_KEY = "lumiere-install-tab";

  function fallbackCopy(text) {
    var area = document.createElement("textarea");
    area.value = text;
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    try {
      document.execCommand("copy");
    } finally {
      document.body.removeChild(area);
    }
  }

  function copyText(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      return navigator.clipboard.writeText(text).catch(function () {
        fallbackCopy(text);
      });
    }
    fallbackCopy(text);
    return Promise.resolve();
  }

  function readSavedTab() {
    try {
      return window.localStorage.getItem(STORAGE_KEY);
    } catch (_) {
      return null;
    }
  }

  function saveTab(id) {
    try {
      window.localStorage.setItem(STORAGE_KEY, id);
    } catch (_) {}
  }

  function initSwitcher(root) {
    var tabs = Array.prototype.slice.call(root.querySelectorAll(".install-tab"));
    var cmdEl = root.querySelector(".install-tabs-cmd");
    var copyBtn = root.querySelector(".install-tabs-copy");
    if (!tabs.length || !cmdEl) return;

    var copyTimer = null;

    function selectTab(tab, persist) {
      tabs.forEach(function (t) {
        var active = t === tab;
        t.setAttribute("aria-selected", active ? "true" : "false");
        t.tabIndex = active ? 0 : -1;
      });
      cmdEl.textContent = tab.getAttribute("data-cmd") || "";
      if (persist && tab.getAttribute("data-tab")) {
        saveTab(tab.getAttribute("data-tab"));
      }
    }

    tabs.forEach(function (tab, index) {
      tab.addEventListener("click", function () {
        selectTab(tab, true);
      });
      tab.addEventListener("keydown", function (event) {
        if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) {
          return;
        }
        var next = null;
        if (event.key === "ArrowRight") next = tabs[(index + 1) % tabs.length];
        else if (event.key === "ArrowLeft") next = tabs[(index - 1 + tabs.length) % tabs.length];
        else if (event.key === "Home") next = tabs[0];
        else if (event.key === "End") next = tabs[tabs.length - 1];
        if (next) {
          event.preventDefault();
          event.stopPropagation();
          selectTab(next, true);
          next.focus();
        }
      });
    });

    var saved = readSavedTab();
    var initial = tabs.find(function (t) {
      return t.getAttribute("data-tab") === saved;
    }) || tabs[0];
    selectTab(initial, false);

    if (copyBtn) {
      copyBtn.addEventListener("click", function () {
        copyText(cmdEl.textContent || "").then(function () {
          copyBtn.classList.add("is-copied");
          if (copyTimer) clearTimeout(copyTimer);
          copyTimer = setTimeout(function () {
            copyBtn.classList.remove("is-copied");
          }, 1500);
        });
      });
    }
  }

  document.querySelectorAll("[data-install-tabs]").forEach(initSwitcher);
})();
