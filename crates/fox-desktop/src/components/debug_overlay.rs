use dioxus::events::eval;
use dioxus::prelude::*;

use crate::state::AppState;

pub const DEBUG_OVERLAY_JS: &str = r#"
(function () {
  if (window.__rfDebugInstalled) return;
  window.__rfDebugInstalled = true;

  setTimeout(function () {
    var tooltip = document.createElement('div');
    tooltip.id = 'rf-debug-tooltip';
    tooltip.style.cssText =
      'position:fixed;z-index:999999;background:#1a1a2e;color:#e2e8f0;' +
      'font:11px/1.5 monospace;padding:6px 10px;border-radius:6px;' +
      'border:1px solid rgba(255,87,34,.5);box-shadow:0 4px 12px rgba(0,0,0,.4);' +
      'pointer-events:none;max-width:360px;white-space:pre-wrap;word-break:break-all;display:none;';
    document.body.appendChild(tooltip);

    var panel = document.createElement('div');
    panel.id = 'rf-debug-panel';
    panel.style.cssText =
      'position:fixed;bottom:16px;right:16px;z-index:999998;' +
      'background:#1a1a2e;color:#e2e8f0;' +
      'font:11px/1.5 monospace;padding:10px 12px;border-radius:8px;' +
      'border:1px solid rgba(255,87,34,.5);box-shadow:0 6px 18px rgba(0,0,0,.5);' +
      'max-width:400px;white-space:pre-wrap;word-break:break-all;display:none;' +
      'pointer-events:auto;';
    document.body.appendChild(panel);

    function isOwn(el) {
      while (el) {
        if (el.id === 'rf-debug-tooltip' || el.id === 'rf-debug-panel' || el.id === 'rf-debug-badge') return true;
        if (el.classList && el.classList.contains('rf-debug-badge')) return true;
        el = el.parentElement;
      }
      return false;
    }

    function buildPath(el) {
      var parts = [];
      var node = el, depth = 0;
      while (node && node.tagName && depth < 8) {
        var p = node.tagName.toLowerCase();
        if (node.id) p += '#' + node.id;
        else if (node.className) {
          var c = typeof node.className === 'string'
            ? node.className.trim().split(/\s+/).filter(Boolean)
            : Array.from(node.className).filter(Boolean);
          if (c.length) p += '.' + c[0];
        }
        parts.unshift(p);
        node = node.parentNode;
        depth++;
      }
      return parts.join(' > ');
    }

    function buildInfo(el) {
      var tag = el.tagName ? el.tagName.toLowerCase() : 'text';
      var id = el.id ? el.id : '(none)';
      var cls = el.className
        ? (typeof el.className === 'string' ? el.className.trim() : Array.from(el.className).join(' '))
        : '(none)';
      var rect = el.getBoundingClientRect();
      var path = buildPath(el);
      return {
        html: '<span style="color:#7dd3fc;font-weight:700;">&lt;' + tag + '&gt;</span><br>' +
          '<span style="color:#fbbf24">#</span>' + id + '<br>' +
          '<span style="color:#86efac">.</span>' + cls + '<br>' +
          '<span style="color:#c084fc">pos:</span> (' + Math.round(rect.left) + ', ' + Math.round(rect.top) + ')<br>' +
          '<span style="color:#fb923c">size:</span> ' + Math.round(rect.width) + ' x ' + Math.round(rect.height) + '<br>' +
          '<span style="color:#94a3b8">path:</span> ' + path,
        text: '[' + tag + '] id="' + id + '" class="' + cls + '" pos=(' + Math.round(rect.left) + ',' + Math.round(rect.top) + ') size=' + Math.round(rect.width) + 'x' + Math.round(rect.height) + ' path=' + path
      };
    }

    function positionTooltip(mx, my) {
      tooltip.style.visibility = 'hidden';
      tooltip.style.display = 'block';
      var w = tooltip.offsetWidth;
      var h = tooltip.offsetHeight;
      var tx = mx + 16;
      var ty = my + 16;
      if (tx + w > window.innerWidth - 16) tx = mx - w - 16;
      if (tx < 8) tx = 8;
      if (ty + h > window.innerHeight - 16) ty = my - h - 16;
      if (ty < 8) ty = 8;
      tooltip.style.left = tx + 'px';
      tooltip.style.top = ty + 'px';
      tooltip.style.visibility = 'visible';
    }

    var COLORS = ['#ef4444','#22c55e','#3b82f6','#a855f7','#f59e0b'];

    window.__rfDebugApply = function (active) {
      document.querySelectorAll('body *').forEach(function (el) {
        if (isOwn(el)) return;
        el.style.outline = active ? '1px solid ' + COLORS[Math.floor(Math.random()*COLORS.length)] : '';
      });
    };

    function showTooltipAt(x, y) {
      var el = document.elementFromPoint(x, y);
      if (!el || isOwn(el)) { tooltip.style.display = 'none'; return; }
      var info = buildInfo(el);
      tooltip.innerHTML = info.html;
      positionTooltip(x, y);
    }

    window.__rfShowTooltip = function (xy) {
      var parts = xy.split('|');
      showTooltipAt(parseInt(parts[0]), parseInt(parts[1]));
    };

    function doInspect(el) {
      if (!el || isOwn(el)) return;
      var tag = el.tagName ? el.tagName.toLowerCase() : '';
      if (['button','input','select','textarea','a','option'].indexOf(tag) !== -1) return;
      if (el.isContentEditable) return;
      var info = buildInfo(el);
      try { navigator.clipboard.writeText(info.text); } catch (e) {}
      panel.innerHTML = info.html + '<br><br><span style="color:#22c55e;font-size:10px;">已复制到剪贴板  双击再次复制</span>';
      panel.style.display = 'block';
      panel.ondblclick = function () {
        try { navigator.clipboard.writeText(info.text); } catch (e) {}
      };
    }

    window.__rfInspect = function (xy) {
      var parts = xy.split('|');
      doInspect(document.elementFromPoint(parseInt(parts[0]), parseInt(parts[1])));
    };

    window.__rfHidePanel = function () { panel.style.display = 'none'; };

    window.__rfHideTooltip = function () { tooltip.style.display = 'none'; };

    document.body.addEventListener('mousemove', function (e) {
      if (document.body.getAttribute('data-rf-debug') !== '1') return;
      showTooltipAt(e.clientX, e.clientY);
    }, false);

    document.body.addEventListener('click', function (e) {
      if (document.body.getAttribute('data-rf-debug') !== '1') return;
      doInspect(e.target);
    }, true);

    var prev = true;
    var obs = new MutationObserver(function () {
      var active = document.body.getAttribute('data-rf-debug') === '1';
      if (prev !== active) {
        if (active) { window.__rfDebugApply(true); }
        else {
          window.__rfDebugApply(false);
          panel.style.display = 'none';
          tooltip.style.display = 'none';
        }
        prev = active;
      }
    });
    obs.observe(document.body, { attributes: true, attributeFilter: ['data-rf-debug'] });
  }, 0);
})();
"#;

#[component]
pub fn DebugOverlay() -> Element {
    let state = use_context::<AppState>();

    let _ = use_hook(|| {
        eval(DEBUG_OVERLAY_JS);
    });

    if !*state.debug_mode.read() {
        return None;
    }

    rsx! {
        div {
            class: "rf-debug-badge",
            title: "点击关闭调试模式",
            onclick: move |_| {
                let mut dm = state.debug_mode;
                dm.set(false);
                eval("document.body.removeAttribute('data-rf-debug'); if (window.__rfDebugApply) window.__rfDebugApply(false); if (window.__rfHidePanel) window.__rfHidePanel();");
            },
            "DEBUG  Ctrl+Shift+D"
        }
    }
}
