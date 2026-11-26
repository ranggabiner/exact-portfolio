// Lumen's native views on the web: lighter stand-ins for the Swift ones in
// modules/apple, so the same screens can be driven and checked in a browser.
export const abi = 1;
const TAGS = ['lumen-aurora', 'lumen-hero', 'lumen-physics', 'lumen-confetti', 'lumen-ring'];
export const roster = Object.fromEntries(TAGS.map((t) => [t, { snapshot: false }]));

const still = () => globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
const PALETTES = {
  dawn: ['#3a22a8', '#8d5bea', '#e0609a'], dusk: ['#2a1670', '#6a2e8c', '#b0417a'],
  violet: ['#2d1b8f', '#6c4be8', '#3a2bb0'], calm: ['#0f3a63', '#1d6f86', '#3b2f9a'],
  rose: ['#5a1e6e', '#b83c78', '#4b2fd6'], focus: ['#241670', '#4b2fd6', '#14507a'],
  celebrate: ['#7c5cff', '#ff6fa8', '#ffb457'],
};
const TINTS = ['#7c5cff', '#b83c78', '#1d8f86', '#c9772b', '#4b2fd6'];

function el(tag, css, text) {
  const node = document.createElement(tag);
  if (css) node.style.cssText = css;
  if (text !== undefined) node.textContent = text;
  return node;
}

const draw = {
  'lumen-aurora'(h, props) {
    const [a, b, c] = PALETTES[props.palette] ?? PALETTES.dawn;
    h.box.style.transition = 'background 900ms ease';
    h.box.style.background = `radial-gradient(90% 55% at 15% 0%, ${a} 0%, transparent 70%), radial-gradient(80% 50% at 95% 20%, ${b} 0%, transparent 70%), radial-gradient(90% 50% at 60% 55%, ${c}99 0%, transparent 70%), #0b0820`;
  },
  'lumen-hero'(h, props) {
    const places = [[8, 12, -7], [46, 30, 5], [14, 56, 4], [48, 76, -5]];
    h.box.replaceChildren(...(props.labels ?? '').split('|').slice(0, 4).map((label, i) => {
      const [x, y, r] = places[i];
      return el('div', `position:absolute;left:${x}%;top:${y}%;transform:rotate(${r}deg);padding:12px 14px;border-radius:18px;background:rgba(255,255,255,.14);border:1px solid rgba(255,255,255,.3);color:#fff;font:600 15px system-ui;white-space:nowrap;box-shadow:0 12px 30px rgba(0,0,0,.35)`, `${i < 1 ? '●' : '○'}  ${label}`);
    }));
  },
  'lumen-physics'(h, props) {
    if (h.items === props.items) return;
    h.items = props.items;
    h.box.replaceChildren(...(props.items ?? '').split('|').map((label, i) => {
      const bubble = el('button', `position:absolute;left:${(i * 37) % 62}%;top:${88 - ((i * 53) % 78)}%;padding:10px 16px;border-radius:999px;border:1px solid rgba(255,255,255,.35);background:${TINTS[i % TINTS.length]};color:#fff;font:600 14px system-ui;transform:rotate(${((i * 29) % 30) - 15}deg);transition:transform .2s,opacity .2s;pointer-events:auto`, label);
      bubble.type = 'button';
      bubble.tabIndex = -1;
      bubble.onclick = () => { bubble.style.opacity = '0'; bubble.style.transform = 'scale(1.35)'; };
      return bubble;
    }));
  },
  'lumen-confetti'(h, props) {
    if ((props.burst ?? '0') === h.last || (props.burst ?? '0') === '0') return;
    h.last = props.burst;
    if (props.kind === 'haptic' || props.kind === 'calm' || still()) return;
    const colors = ['#8d6bff', '#ff6fa8', '#ffb457', '#5ee6c4', '#5aa9ff'];
    for (let i = 0; i < 60; i++) {
      const piece = el('div', `position:absolute;top:-12px;left:${(i * 61) % 100}%;width:8px;height:12px;border-radius:2px;background:${colors[i % colors.length]}`);
      h.box.append(piece);
      piece.animate?.([{ transform: 'translateY(0) rotate(0)', opacity: 1 }, { transform: `translateY(${420 + ((i * 47) % 300)}px) rotate(${360 + i * 23}deg)`, opacity: 0 }], { duration: 1600 + ((i * 31) % 900), easing: 'ease-in', fill: 'forwards' }).finished.then(() => piece.remove());
    }
  },
  'lumen-ring'(h, props) {
    const progress = Math.max(0, Math.min(1, Number(props.progress) || 0));
    const thick = props.thick === '1';
    const stroke = thick ? 5.5 : 12, r = 50 - stroke / 2, length = 2 * Math.PI * r;
    if (!h.arc) {
      const ns = 'http://www.w3.org/2000/svg';
      const svg = document.createElementNS(ns, 'svg');
      svg.setAttribute('viewBox', '0 0 100 100');
      svg.style.cssText = 'width:100%;height:100%';
      const circle = (color) => {
        const c = document.createElementNS(ns, 'circle');
        for (const [k, v] of Object.entries({ cx: 50, cy: 50, fill: 'none', stroke: color, 'stroke-linecap': 'round', transform: 'rotate(-90 50 50)' })) c.setAttribute(k, v);
        svg.append(c);
        return c;
      };
      h.track = circle('rgba(124,92,255,.2)');
      h.arc = circle('#8d6bff');
      h.arc.style.transition = 'stroke-dashoffset 600ms ease';
      h.text = document.createElementNS(ns, 'text');
      for (const [k, v] of Object.entries({ x: 50, y: 61, 'text-anchor': 'middle', 'font-size': 30, 'font-weight': 800, 'font-family': 'system-ui', fill: 'currentColor' })) h.text.setAttribute(k, v);
      svg.append(h.text);
      h.box.replaceChildren(svg);
    }
    for (const c of [h.track, h.arc]) { c.setAttribute('r', r); c.setAttribute('stroke-width', stroke); }
    h.arc.setAttribute('stroke-dasharray', length);
    h.arc.setAttribute('stroke-dashoffset', length * (1 - progress));
    h.arc.style.opacity = progress > 0 ? 1 : 0;
    h.text.textContent = `${props.figure ?? ''}${props.suffix ?? ''}`;
  },
};

export function create(tag, element, json) {
  const root = element.shadowRoot ?? element.attachShadow({ mode: 'open' });
  const box = el('div', 'position:relative;width:100%;height:100%;overflow:hidden;pointer-events:none');
  root.replaceChildren(box);
  const h = { tag, box };
  draw[tag](h, JSON.parse(json));
  return h;
}

export function setProps(h, json) {
  draw[h.tag](h, JSON.parse(json));
}

export function snapshot() {}

export function destroy(h) {
  h.box.replaceChildren();
}
