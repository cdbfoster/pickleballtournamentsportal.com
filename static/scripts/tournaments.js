const viewLimitIncrement = 6;

const abbreviations = {
  "alabama": "AL",
  "alaska": "AK",
  "american samoa": "AS",
  "arizona": "AZ",
  "arkansas": "AR",
  "california": "CA",
  "colorado": "CO",
  "connecticut": "CT",
  "delaware": "DE",
  "district of columbia": "DC",
  "federated states of micronesia": "FM",
  "florida": "FL",
  "georgia": "GA",
  "guam": "GU",
  "hawaii": "HI",
  "idaho": "ID",
  "illinois": "IL",
  "indiana": "IN",
  "iowa": "IA",
  "kansas": "KS",
  "kentucky": "KY",
  "louisiana": "LA",
  "maine": "ME",
  "marshall islands": "MH",
  "maryland": "MD",
  "massachusetts": "MA",
  "michigan": "MI",
  "minnesota": "MN",
  "mississippi": "MS",
  "missouri": "MO",
  "montana": "MT",
  "nebraska": "NE",
  "nevada": "NV",
  "new hampshire": "NH",
  "new jersey": "NJ",
  "new mexico": "NM",
  "new york": "NY",
  "north carolina": "NC",
  "north dakota": "ND",
  "northern mariana islands": "MP",
  "ohio": "OH",
  "oklahoma": "OK",
  "oregon": "OR",
  "palau": "PW",
  "pennsylvania": "PA",
  "puerto rico": "PR",
  "rhode island": "RI",
  "south carolina": "SC",
  "south dakota": "SD",
  "tennessee": "TN",
  "texas": "TX",
  "utah": "UT",
  "vermont": "VT",
  "virgin islands": "VI",
  "virginia": "VA",
  "washington": "WA",
  "west virginia": "WV",
  "wisconsin": "WI",
  "wyoming": "WY",
};

var tournamentListings = null;
var captcha = null;

fetch("/tournaments/fetch")
  .then(response => response.json())
  .then(data => {
    if ("tournaments" in data) {
      tournamentListings = data.tournaments;
    } else if ("captcha" in data) {
      captcha = data.captcha;
    } else {
      console.error("Error: " + data.error.reason);
    }
    m.redraw();
  })
  .catch(error => console.error(error));

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

class Main {
  constructor() {
    this.nameFilter = "";
    this.locationFilter = "";
    this.locationMatches = [];

    this.showClosed = true;
    this.showOngoing = true;
    this.showFuture = true;
    this.showPast = true;

    this.viewLimits = {
      ongoing: viewLimitIncrement,
      future: viewLimitIncrement,
      past: viewLimitIncrement,
    };
  }

  oncreate(vnode) {
    const filterEvent = new Event("filter", { bubbles: true });

    vnode.dom.querySelector("#name-filter").addEventListener("input", (event) => {
      this.nameFilter = event.target.value.toLowerCase();

      event.target.dispatchEvent(filterEvent);
    });

    vnode.dom.querySelector("#location-filter").addEventListener("input", (event) => {
      this.locationFilter = event.target.value.toLowerCase();

      this.locationMatches = Object.keys(abbreviations)
        .filter(l => this.locationFilter.includes(l))
        .map(l => abbreviations[l]);

      this.locationMatches = this.locationMatches.concat(
        Object.values(abbreviations)
          .filter(l => new RegExp(`\\b${l.toLocaleLowerCase()}\\b`).test(this.locationFilter)),
      );

      event.target.dispatchEvent(filterEvent);
    });

    vnode.dom.querySelector("#show-ongoing").addEventListener("change", (event) => {
      this.showOngoing = event.target.checked;
      event.target.dispatchEvent(filterEvent);
    });

    vnode.dom.querySelector("#show-future").addEventListener("change", (event) => {
      this.showFuture = event.target.checked;
      event.target.dispatchEvent(filterEvent);
    });

    vnode.dom.querySelector("#show-past").addEventListener("change", (event) => {
      this.showPast = event.target.checked;
      if (this.showPast) {
        this.showClosed = true;
      }
      event.target.dispatchEvent(filterEvent);
    });

    vnode.dom.querySelector("#show-closed").addEventListener("change", (event) => {
      this.showClosed = event.target.checked;
      if (!this.showClosed) {
        this.showPast = false;
      }
      event.target.dispatchEvent(filterEvent);
    });

    vnode.dom.addEventListener("filter", (event) => {
      // Reset view limits to initial values
      this.viewLimits = {
        ongoing: viewLimitIncrement,
        future: viewLimitIncrement,
        past: viewLimitIncrement,
      };
      m.redraw();
      event.stopPropagation();
    })
  }

  view() {
    let currentDate = new Date();
    let localYear = currentDate.getFullYear();
    let localMonth = (currentDate.getMonth() + 1).toString().padStart(2, "0");
    let localDay = currentDate.getDate().toString().padStart(2, "0");
    currentDate = `${localYear}-${localMonth}-${localDay}`;

    const isOngoing = t => t.startDate <= currentDate && t.endDate >= currentDate;
    const isFuture = t => t.startDate > currentDate;
    const isPast = t => t.endDate < currentDate;

    function sortByDate(a, b, ascending=true) {
      if (a.startDate < b.startDate) {
        return -1 * (ascending ? 1 : -1);
      } else if (a.startDate > b.startDate) {
        return 1 * (ascending ? 1 : -1);
      } else {
        return a.name.toLowerCase().localeCompare(b.name.toLowerCase(), "en", { sensitivity: "base", ignorePunctuation: true });
      }
    }

    let filter = (list) => {
      if (!this.showOngoing) {
        list = list.filter(t => !isOngoing(t));
      }
      if (!this.showFuture) {
        list = list.filter(t => !isFuture(t));
      }
      if (!this.showPast) {
        list = list.filter(t => !isPast(t));
      }
      if (!this.showClosed) {
        list = list.filter(t => {
          return typeof(t.registrationStatus) === "object" && (
            Object.keys(t.registrationStatus)[0] == "open" ||
            Object.keys(t.registrationStatus)[0] == "openSoon"
          );
        });
      }

      list.sort((a, b) => {
        if (a.endDate < currentDate && b.endDate < currentDate) {
          return sortByDate(a, b, false);
        } else if (a.endDate < currentDate) {
          return -1;
        } else if (b.endDate < currentDate) {
          return 1;
        } else {
          return sortByDate(a, b);
        }
      });

      if (this.locationFilter != "") {
        if (this.locationMatches.length != 0) {
          list = list.filter(t => {
            for (let l of this.locationMatches) {
              if (new RegExp(`\\b${l}\\b`).test(t.location)) {
                return true;
              }
            }
            return false;
          });
        } else {
          let distances = list.map(t => [t, fuzzyCompare(t.location.toLowerCase(), this.locationFilter)]);
          distances.sort((a, b) => a[1] - b[1]);
          list = distances.filter(a => a[1] <= distances[0][1] + 1).map(a => a[0]);
        }
      }

      if (this.nameFilter != "") {
        let distances = list.map(t => [t, fuzzyCompare(t.name.toLowerCase(), this.nameFilter)]);
        distances.sort((a, b) => a[1] - b[1]);
        list = distances.filter(a => a[1] <= distances[0][1] + 1).map(a => a[0]);
      }

      return list;
    };

    // tournamentListings is defined in another script tag.
    let filteredTournaments = filter(tournamentListings);

    let ongoingTournaments = filteredTournaments.filter(isOngoing);
    let futureTournaments = filteredTournaments.filter(isFuture);
    let pastTournaments = filteredTournaments.filter(isPast);

    return [
      m("section#filter", { key: "filter"}, [
        m("h2", "Tournament Search"),
        m("label#name-filter-label", m("input#name-filter", { type: "text", placeholder: "Filter by name" })),
        m("label#location-filter-label", m("input#location-filter", { type: "text", placeholder: "Filter by location" })),
        m("ul#view-toggles", [
          m("li", m("label", [m("input#show-ongoing", { type: "checkbox", checked: this.showOngoing }), "Show ongoing"])),
          m("li", m("label", [m("input#show-future", { type: "checkbox", checked: this.showFuture }), "Show future"])),
          m("li", m("label", [m("input#show-past", { type: "checkbox", checked: this.showPast }), "Show past"])),
          m("li", m("label", [m("input#show-closed", { type: "checkbox", checked: this.showClosed }), "Show closed"])),
        ]),
      ]),
      ...(ongoingTournaments.length > 0 ? [
        m("section#ongoing-tournaments.tournament-list", { key: "ongoing-tournaments" }, [
          m("h2", "Ongoing Tournaments"),
          m("ul", ongoingTournaments.slice(0, this.viewLimits.ongoing).map(t => m("li", { key: t.id }, m(TournamentListing, { tournament: t })))),
          ...(ongoingTournaments.length > this.viewLimits.ongoing ? [
            m("button.load-more", { onclick: () => this.viewLimits.ongoing += viewLimitIncrement }, "Load more results..."),
          ] : []),
        ]),
      ] : []),
      ...(futureTournaments.length > 0 ? [
        m("section#future-tournaments.tournament-list", { key: "future-tournaments" }, [
          m("h2", "Future Tournaments"),
          m("ul", futureTournaments.slice(0, this.viewLimits.future).map(t => m("li", { key: t.id }, m(TournamentListing, { tournament: t })))),
          ...(futureTournaments.length > this.viewLimits.future ? [
            m("button.load-more", { onclick: () => this.viewLimits.future += viewLimitIncrement }, "Load more results..."),
          ] : []),
        ]),
      ] : []),
      ...(pastTournaments.length > 0 ? [
        m("section#past-tournaments.tournament-list", { key: "past-tournaments" }, [
          m("h2", "Past Tournaments"),
          m("ul", pastTournaments.slice(0, this.viewLimits.past).map(t => m("li", { key: t.id }, m(TournamentListing, { tournament: t })))),
          ...(pastTournaments.length > this.viewLimits.past ? [
            m("button.load-more", { onclick: () => this.viewLimits.past += viewLimitIncrement }, "Load more results..."),
          ] : []),
        ]),
      ] : []),
    ];
  }
}

class TournamentListing {
  view(vnode) {
    let tournament = vnode.attrs.tournament;

    let datesString = tournament.startDate != tournament.endDate ? `${printDate(tournament.startDate)} - ${printDate(tournament.endDate)}` : printDate(tournament.startDate);
    return m("div.tournament-listing", [
      m("h3.name", m("a", { href: `https://www.pickleballtournaments.com/tournamentinfo.pl?tid=${tournament.id}` }, m.trust(tournament.name))),
      m("p.location", m.trust(tournament.location)),
      m("p.dates", datesString),
      m("div.logo", tournament.logoUrl !== null ? [m(LazyImage, { src: tournament.logoUrl })] : []),
      m(RegistrationStatus, { status: tournament.registrationStatus }),
      m("ul.tags", tournament.tagUrls.map(t => m("li", m(LazyImage, { src: t })))),
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

let main = document.querySelector("main");

m.mount(main, {
  view: function () {
    return captcha !== null ? m(Captcha) : (tournamentListings !== null ? m(Main) : m(Loading));
  },
});