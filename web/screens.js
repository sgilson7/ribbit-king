// The screens between holes: setup, the hole's card, the scorecard and the
// final card. Each draws the core's screen() and sends button presses back
// as calls; none decides anything.
import { fill } from "./hud.js";

const el = (id) => document.getElementById(id);
const make = (tag, cls, text) => {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (text !== undefined) n.textContent = text;
  return n;
};
const face = (frog) => {
  const img = make("img");
  img.src = `art/frogs/${frog.id}.png`;
  img.alt = frog.name;
  return img;
};

export function makeScreens(game, copy, refresh) {
  const act = (fn) => () => {
    fn();
    refresh();
  };

  return {
    setup(s) {
      const slots = el("slots");
      slots.replaceChildren();
      s.slots.forEach((slot, i) => {
        const box = make("div", `slot ${slot.kind}`);
        const kind = make("button", "kind", copy[`slot_${slot.kind}`]);
        kind.title = copy.slot_toggle;
        kind.addEventListener("click", act(() => game.cycle_slot(i)));
        box.append(kind);
        if (slot.kind !== "off") {
          const img = face(slot.frog);
          img.addEventListener("click", act(() => game.cycle_frog(i, true)));
          const row = make("div", "row");
          const prev = make("button", "arrow", "◀");
          prev.setAttribute("aria-label", copy.frog_prev);
          prev.addEventListener("click", act(() => game.cycle_frog(i, false)));
          const next = make("button", "arrow", "▶");
          next.setAttribute("aria-label", copy.frog_next);
          next.addEventListener("click", act(() => game.cycle_frog(i, true)));
          const name = make("span", "frog-name", slot.frog.name);
          name.style.color = slot.frog.colour;
          row.append(prev, name, next);
          box.append(img, row, make("div", "frog-kind", slot.frog.kind));
        }
        slots.append(box);
      });
      el("course-name").textContent = fill(copy[s.course.key], s.course);
      el("best-round").textContent = s.best_round === null ? "" : fill(copy.best_round, { n: s.best_round });
    },

    intro(s) {
      el("intro-art").src = `art/holes/${s.theme}.png`;
      el("i-of").textContent = fill(copy.hole_of, s);
      el("i-name").textContent = s.name;
      el("i-par").textContent = fill(copy.par, s);
      el("i-length").textContent = fill(copy.length, { m: s.length });
      el("i-best").textContent = s.best === null || s.best === undefined ? "" : fill(copy.best_here, { n: s.best });
      const frogs = el("i-frogs");
      frogs.replaceChildren(...s.players.map(face));
    },

    card(s, final) {
      el("crown-art").hidden = !final;
      el("c-title").textContent = final ? copy.final_title : copy.card_title;
      const names = s.winners.map((w) => w.name).join(" & ");
      el("c-sub").textContent = final
        ? fill(s.winners.length === 1 ? copy.winner : copy.winners, { who: names })
        : fill(copy.card_after, { n: s.played, of: s.holes.length });
      const table = el("c-table");
      table.replaceChildren();
      const head = table.insertRow();
      head.append(make("th", "", copy.card_frog));
      for (const h of s.holes) {
        const th = make("th", "", h.name);
        th.append(make("span", "word", fill(copy.par, h)));
        head.append(th);
      }
      head.append(make("th", "", copy.card_total), make("th", "", copy.card_flies));
      s.rows.forEach((r, k) => {
        const row = table.insertRow();
        if (r.total === s.rows[0].total) row.className = "lead";
        const who = make("td", "frog");
        who.append(face(r.frog), make("span", "", r.frog.name));
        if (r.cpu) who.append(make("span", "cpu-tag", copy.slot_cpu));
        row.append(who);
        s.holes.forEach((h, i) => {
          const done = i < r.scores.length;
          const td = make("td", "", done ? r.scores[i] : "–");
          if (done) {
            const word = r.words[i];
            td.className = word === "par" ? "" : word === "bogey" || word === "double_bogey" || word === "over_par" ? "over" : "under";
            td.append(make("span", "word", copy.words[word]));
          }
          row.append(td);
        });
        const total = make("td", "total", r.total);
        total.append(make("span", "word", r.vs));
        row.append(total, make("td", "", r.flies));
        if (k === 0) row.dataset.first = "true";
      });
      el("card-next").textContent = final ? copy.again : s.final ? copy.see_final : copy.next_hole;
    },

    help(controlsRows, helpRows) {
      const dl = el("help-rows");
      dl.replaceChildren();
      for (const [t, d] of helpRows) dl.append(make("dt", "", t), make("dd", "", d));
      const keys = el("help-keys");
      keys.replaceChildren();
      for (const [k, d] of controlsRows) {
        const row = keys.insertRow();
        row.insertCell().textContent = k;
        row.insertCell().textContent = d;
      }
    },
  };
}
