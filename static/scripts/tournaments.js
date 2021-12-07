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
var error = false;

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
  .catch(e => {
    console.error(e);
    error = true;
    m.redraw();
  });

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
        .filter(l => l.startsWith(this.locationFilter))
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
      window.innerWidth <= 800 && document.querySelector("main").scrollIntoView(true);

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
          list = filterArray(this.locationFilter, list, t => t.location);
        }
      }

      if (this.nameFilter != "") {
        list = filterArray(this.nameFilter, list, t => t.name);
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
        m("label#name-filter-label", m("input#name-filter.filter", { type: "text", placeholder: "Filter by name" })),
        m("label#location-filter-label", m("input#location-filter.filter", { type: "text", placeholder: "Filter by location" })),
        m("ul#view-toggles", [
          m("li", m("label", [m("input#show-ongoing", { type: "checkbox", checked: this.showOngoing }), "Show ongoing"])),
          m("li", m("label", [m("input#show-future", { type: "checkbox", checked: this.showFuture }), "Show future"])),
          m("li", m("label", [m("input#show-past", { type: "checkbox", checked: this.showPast }), "Show past"])),
          m("li", m("label", [m("input#show-closed", { type: "checkbox", checked: this.showClosed }), "Show closed"])),
        ]),
      ]),
      ...(filteredTournaments.length > 0 ? [
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
      ] : [m("p", { key: "no-matches" }, "No tournaments match the selected filters")]),
    ];
  }
}

class TournamentListing {
  view(vnode) {
    let tournament = vnode.attrs.tournament;

    let datesString = tournament.startDate != tournament.endDate ? `${printDate(tournament.startDate)} - ${printDate(tournament.endDate)}` : printDate(tournament.startDate);
    return m("div.tournament-listing", [
      m("h3.name", m("a", { href: `/tournament/${tournament.id}` }, m.trust(tournament.name))),
      m("p.location", m.trust(tournament.location)),
      m("p.dates", datesString),
      m("div.logo", tournament.logoUrl !== null ? [m(LazyImage, { src: tournament.logoUrl })] : []),
      m(RegistrationStatus, { status: tournament.registrationStatus }),
      m("ul.tags", tournament.tagUrls.map(t => m("li", m(LazyImage, { src: t })))),
    ]);
  }
}

let main = document.querySelector("main");

m.mount(main, {
  view: function () {
    return error ? m(Error) : (captcha !== null ? m(Captcha) : (tournamentListings !== null ? m(Main) : m(Loading)));
  },
});