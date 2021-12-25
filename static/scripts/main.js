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
        id: vnode.attrs.id,
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
    this.captchaShown = false;
    this.captchaLoaded = false;
  }

  view() {
    return m(
      "div.captcha",
      {
        class: this.captchaShown ? "shown" : undefined,
      },
      this.captchaShown ? [
        m("h1", "Please fill out this captcha"),
        m("div.iframe-shell", [
          m("iframe", {
            src: captcha.url,
            onload: (event) => {
              if (this.captchaLoaded == true) {
                location.reload();
              }
              document.querySelector(".iframe-shell").scrollTo(0, 10000);
              this.captchaLoaded = true;
            },
          }),
        ]),
      ] : [
        m("div", [
          m("h1", "Please fill out this captcha"),
          m("p", "Due to the way this site works, we get captcha'd pretty regularly."),
          m("p", "Once you fill out the captcha, the page will reload."),
        ]),
        m("button.captcha-button", { onclick: () => this.captchaShown = true }, "Show captcha"),
      ],
    );
  }
}

class Error {
  view() {
    return m("div.error", [
      m("p", "Error fetching data"),
      m("p", [
        "Something went wrong while fetching data from ",
        m("a", { href: "https://www.pickleballtournaments.com" }, "PickleballTournaments.com"),
        ". Please try again later.",
      ]),
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

class RegistrationStatus {
  view(vnode) {
    let status = vnode.attrs.status;

    let registrationClass;
    let registrationType;
    let detail;

    if (typeof(status) == "string") {
      if (status == "closed") {
        registrationClass = "closed";
        registrationType = "Closed";
      } else if (status == "notOpen") {
        registrationClass = "closed";
        registrationType = "Not open";
      }
      detail = null;
    } else if (typeof(status) == "object") {
      let s = Object.keys(status)[0];
      if (s == "open") {
        registrationClass = "open";
        registrationType = "Open";
        detail = `Registration closes ${printDate(status[s].deadline)}`;
      } else if (s == "openSoon") {
        registrationClass = "open-soon";
        registrationType = "Opens soon";
        detail = `Registration opens ${printDate(status[s].startDate)} at ${status[s].startTime}`;
      } else if (s == "closedToNew") {
        registrationClass = "closed";
        registrationType = "Closed to new registrations";
        detail = `Payment deadline: ${status[s].paymentDeadline}`;
      }
    }

    return m("div.registration-status", [
      m("p.status", { class: registrationClass }, registrationType),
      ...(detail ? [m("p.detail", detail)] : []),
    ]);
  }
}

function sanitizeString(string) {
  let stripSymbols = /['"]/g;
  let splitSymbols = /[!@#$%^&*()\[\]{}\/?\\;:|<>_=-]/g;
  return string.toLowerCase().replaceAll(stripSymbols, "").replaceAll(splitSymbols, " ").split(/\s+/).filter(s => s.length > 0);
}

function arrayMatches(a, b) {
  return a.every(x => b.some(y => y.startsWith(x)));
}

function filterArray(filter, array, key) {
  filter = sanitizeString(filter);
  if (filter.length == 0) {
    return array;
  }

  return array.filter(o => arrayMatches(filter, sanitizeString(key(o))));
}

function matchesFilter(string, filter) {
  filter = sanitizeString(filter);
  if (filter.length == 0) {
    return false;
  }

  return arrayMatches(filter, sanitizeString(string));
}

function sortArray(array, key) {
  let keys = array.map(x => [key(x), x]);
  return keys.sort().map(k => k[1]);
}

function printDate(isoDate, pretty = false) {
  let utc = new Date(isoDate);
  let local = new Date();
  local.setFullYear(utc.getUTCFullYear());
  local.setMonth(utc.getUTCMonth());
  local.setDate(utc.getUTCDate());

  return local.toLocaleString(
    "default",
    {
      weekday: pretty ? "short" : undefined,
      year: "numeric",
      month: pretty ? "short" : "numeric",
      day: "numeric",
    },
  );
}