// The HUD: draws what the core's hud() reports and decides nothing. Words
// come from data/copy.json; positions on screen come from the core as
// percentages; sounds the core names are played.

const el = (id) => document.getElementById(id);

export function fill(text, values) {
  let out = text || "";
  for (const [k, v] of Object.entries(values)) out = out.split(`{${k}}`).join(String(v));
  return out;
}

export function makeHud(copy, play) {
  let lastSay = 0;
  const tags = new Map();

  function standings(players) {
    const list = el("h-standings");
    list.replaceChildren();
    for (const p of players) {
      const li = document.createElement("li");
      li.className = `${p.active ? "active" : ""} ${p.holed ? "holed" : ""}`;
      const dot = document.createElement("span");
      dot.className = "dot";
      dot.style.background = p.colour;
      const name = document.createElement("span");
      name.textContent = p.cpu ? `${p.name} (${copy.slot_cpu})` : p.name;
      const n = document.createElement("span");
      n.className = "n";
      n.textContent = `${p.strokes}${p.holed ? " ✿" : ""}`;
      const pts = document.createElement("span");
      pts.className = "pts";
      pts.textContent = fill(copy.points, { n: p.points });
      li.append(dot, name, n, pts);
      list.append(li);
    }
  }

  function placeTags(players) {
    const layer = el("tags");
    for (const p of players) {
      let t = tags.get(p.name);
      if (!t) {
        t = document.createElement("div");
        t.className = "tag";
        t.textContent = p.name;
        t.style.borderColor = p.colour;
        layer.append(t);
        tags.set(p.name, t);
      }
      t.hidden = !p.on;
      t.classList.toggle("active", p.active);
      t.style.left = `${p.x}%`;
      t.style.top = `${p.y}%`;
    }
  }

  function toast(say) {
    const words = copy.says[say.key];
    if (!words) return;
    const t = document.createElement("div");
    t.className = say.key === "turn" || say.key === "landed" ? "toast small" : say.key === "points" ? "toast points" : "toast";
    t.textContent = fill(words, { who: say.who, n: say.n });
    t.addEventListener("animationend", () => t.remove());
    el("toasts").append(t);
  }

  return {
    reset() {
      for (const t of tags.values()) t.remove();
      tags.clear();
      el("toasts").replaceChildren();
    },
    update(h) {
      for (const s of h.sounds || []) play(s);
      if (!h.hole) return;
      el("h-hole").textContent = h.hole.name;
      el("h-of").textContent = fill(copy.hole_of, h.hole);
      el("h-par").textContent = fill(copy.par, h.hole);
      const o = h.online;
      el("h-online").hidden = !o;
      if (o) el("h-online").textContent = o.desync !== null ? copy.online_desync : o.room ? fill(copy.online_badge, { room: o.room }) : copy.online_badge_code;
      const p = h.player;
      el("p-face").src = `art/frogs/${p.frog}.png`;
      el("p-name").textContent = p.cpu ? `${p.name} (${copy.slot_cpu})` : p.name;
      el("p-name").style.color = p.colour;
      el("p-stroke").textContent = fill(copy.stroke, { n: p.strokes });
      el("p-flies").textContent = p.tongue ? p.flies : fill(copy.tongue_need, { n: p.flies });
      el("p-points").textContent = fill(copy.points, { n: p.points });
      el("g-tongue").hidden = !p.tongue;
      el("g-jumps").hidden = !p.jumps;
      el("p-jumps").textContent = `${copy.jumps} × ${p.jumps}`;
      el("g-wings").hidden = p.wings === null;
      el("p-wings").textContent = fill(copy.wings, { s: p.wings });
      el("prompt").textContent = fill(copy[h.prompt] || "", { who: p.name });
      standings(h.players);
      placeTags(h.players);
      const lt = el("lotus-tag");
      lt.hidden = !h.lotus.on;
      lt.style.left = `${h.lotus.x}%`;
      lt.style.top = `${h.lotus.y}%`;
      el("lotus-dist").textContent = fill(copy.to_lotus, { m: h.lotus.dist });
      const m = h.meter;
      const box = el("meter-box");
      box.hidden = !m.show;
      box.className = `${m.spinning ? "" : "idle"} ${m.hit || ""}`;
      el("m-bar").setAttribute("d", m.bar);
      el("m-perfect").setAttribute("d", m.perfect);
      el("m-needle").setAttribute("x2", m.needle[0]);
      el("m-needle").setAttribute("y2", m.needle[1]);
      el("m-power").textContent = `${h.power}%`;
      el("m-loft").textContent = copy[h.loft];
      el("m-dist").textContent = fill(copy.to_lotus, { m: h.dist });
      for (const s of h.says) {
        if (s.id > lastSay) {
          lastSay = s.id;
          toast(s);
        }
      }
    },
  };
}
