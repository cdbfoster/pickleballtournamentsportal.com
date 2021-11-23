class Accordion {
  constructor(vnode) {
    this.expanded = vnode.attrs.expanded || false;
  }

  view(vnode) {
    let title = vnode.attrs.title;
    let content = vnode.attrs.content;

    return m(
      "div.accordion",
      {
        class: (vnode.attrs.class ? vnode.attrs.class + " " : "") + (this.expanded ? "expanded" : "collapsed"),
      },
      [
        m(
          "div.title",
          { onclick: () => this.expanded = !this.expanded },
          [
            m("div.expander"),
            m("div.content", title),
          ],
        ),
        m("div.content", this.expanded ? content : []),
      ],
    )
  }
}

class Captcha {
  constructor() {
    this.awaitingResponse = false;
  }

  view() {
    return m("div.captcha", [
      m("h1", "Please fill out this captcha"),
      m("p", [
        "Due to the way this site works, we get captcha'd pretty regularly.",
      ]),
      m("p", [
        "Once you fill out the captcha and are redirected to ",
        m("a", { href: "https://www.pickleballtournaments.com" }, "PickleballTournaments.com"),
        ", you may return to this page and refresh to try again."
      ]),
      this.awaitingResponse ?
        m("button.captcha-button", { onclick: () => location.reload() }, "Refresh") :
        m("a.captcha-button", {
          target: "_blank",
          rel: "noopener noreferrer",
          href: captcha.url,
          onclick: () => this.awaitingResponse = true,
        }, "Continue to captcha"),
    ]);
  }
}

class LazyImage {
  constructor(vnode) {
    this.loaded = false;
    this.observer = new IntersectionObserver(entries => {
      if (entries[0].isIntersecting && !this.loaded) {
        this.loaded = true;
        m.redraw();
      }
    });
  }

  oncreate(vnode) {
    this.observer.observe(vnode.dom);
  }

  view(vnode) {
    return m("img", {
      src: this.loaded ? vnode.attrs.src : "",
    });
  }
}

class Loading {
  view() {
    return m("div.loading", [
      m("p", "Fetching data from PickleballTournaments.com"),
      m("div.loading-indicator", [
        m("div.loading-indicator-pip"),
        m("div.loading-indicator-pip"),
        m("div.loading-indicator-pip"),
      ]),
    ]);
  }
}

// Adapted from https://github.com/gustf/js-levenshtein
function levenshteinDistance(a, b) {
  if (a === b) {
    return 0;
  }

  if (a.length > b.length) {
    [a, b] = [b, a];
  }

  let lengthA = a.length;
  let lengthB = b.length;

  while (lengthA > 0 && (a.charCodeAt(lengthA - 1) === b.charCodeAt(lengthB - 1))) {
    lengthA--;
    lengthB--;
  }

  let offset = 0;
  while (offset < lengthA && (a.charCodeAt(offset) === b.charCodeAt(offset))) {
    offset++;
  }

  lengthA -= offset;
  lengthB -= offset;

  if (lengthA === 0 || lengthB < 3) {
    return lengthB;
  }

  let vector = [];
  for (let y = 0; y < lengthA; y++) {
    vector.push(y + 1);
    vector.push(a.charCodeAt(offset + y));
  }

  function min(d0, d1, d2, bx, ay)
  {
    return d0 < d1 || d2 < d1
        ? d0 > d2
            ? d2 + 1
            : d0 + 1
        : bx === ay
            ? d1
            : d1 + 1;
  }

  let len = vector.length - 1;
  let x = 0;
  let d0, d1, d2, d3;
  let dd;

  for (; x < lengthB - 3;) {
    let bx0 = b.charCodeAt(offset + (d0 = x));
    let bx1 = b.charCodeAt(offset + (d1 = x + 1));
    let bx2 = b.charCodeAt(offset + (d2 = x + 2));
    let bx3 = b.charCodeAt(offset + (d3 = x + 3));
    dd = (x += 4);
    for (let y = 0; y < len; y += 2) {
      let dy = vector[y];
      let ay = vector[y + 1];
      d0 = min(dy, d0, d1, bx0, ay);
      d1 = min(d0, d1, d2, bx1, ay);
      d2 = min(d1, d2, d3, bx2, ay);
      dd = min(d2, d3, dd, bx3, ay);
      vector[y] = dd;
      d3 = d2;
      d2 = d1;
      d1 = d0;
      d0 = dy;
    }
  }

  for (; x < lengthB;) {
    let bx0 = b.charCodeAt(offset + (d0 = x));
    dd = ++x;
    for (let y = 0; y < len; y += 2) {
      let dy = vector[y];
      vector[y] = dd = min(dy, d0, dd, bx0, vector[y + 1]);
      d0 = dy;
    }
  }

  return dd;
}

function fuzzyCompare(a, b) {
  let minimumDistance = b.length;
  for (let i = 0; i <= Math.max(0, a.length - b.length); i++) {
    minimumDistance = Math.min(minimumDistance, levenshteinDistance(a.substring(i, i + b.length), b));
    if (minimumDistance == 0) {
      break;
    }
  }
  return minimumDistance;
}

function printDate(isoDate) {
  let utc = new Date(isoDate);
  let local = new Date();
  local.setFullYear(utc.getUTCFullYear());
  local.setMonth(utc.getUTCMonth());
  local.setDate(utc.getUTCDate());
  return local.toLocaleString(
    "default",
    {
      year: "numeric",
      month: "numeric",
      day: "numeric",
    },
  );
}