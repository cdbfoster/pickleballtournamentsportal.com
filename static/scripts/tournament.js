var tournamentData = null;
var captcha = null;
var error = false;

fetch(`/tournament/${tournamentId}/data`)
  .then(response => response.json())
  .then(data => {
    if ("tournamentData" in data) {
      tournamentData = data.tournamentData;
      document.title = `${tournamentData.listing.name} | PickleballTournamentsPortal.com`;
    } else if ("captcha" in data) {
      captcha = data.captcha;
    } else {
      console.error("Error: " + data.error.reason);

      if (data.error.reason == "tournament not found") {
        window.location.replace("/not-found");
      }
    }
    m.redraw();
  })
  .catch(e => {
    console.error(e);
    error = true;
    m.redraw();
  });

class Main {
  view() {

    let listing = tournamentData.listing;
    let datesString = listing.startDate != listing.endDate ?
      `<span class="date">${printDate(listing.startDate, true)}</span> - <span class="date">${printDate(listing.endDate, true)}</span>` :
      `<span class="date">${printDate(listing.startDate, true)}</span>`;

    return [
      m("section#listing", [
        m("a#original-link", {
          href: `https://www.pickleballtournaments.com/tournamentinfo.pl?tid=${tournamentId}`,
          rel: "noreferrer",
        }, "View on PickleballTournaments.com"),
        tournamentData.listing.logoUrl !== null ? [m("img", { src: tournamentData.listing.logoUrl })] : [],
        m("h2.name", tournamentData.listing.name),
        m("p.dates", m.trust(datesString)),
        m(RegistrationStatus, { status: tournamentData.listing.registrationStatus }),
      ]),
      m("section#info", tournamentData.info.map(info => {
        let title, content;
        [title, content] = info;
        return m(Accordion, {
          title: m("h3", title),
          content: m.trust(content),
        });
      })),
      m("section#data", [
        tournamentData.schedule.length != 0 ? [
          m(Accordion, {
            id: "schedule",
            title: m("h3", "Schedule"),
            content: m(Schedule),
          }),
        ] : [],
        tournamentData.eventGroups.some(g => g.events.length != 0) ? [
          m(Accordion, {
            id: "events",
            title: m("h3", "Events"),
            content: m(Events),
          }),
        ] : [],
        tournamentData.players.length != 0 ? [
          m(Accordion, {
            id: "players",
            title: m("h3", "Players"),
            content: m(Players),
          }),
        ] : [],
      ]),
    ];
  }
}

class Schedule {
  constructor() {
    this.filter = "";
  }

  oncreate(vnode) {
    vnode.dom.querySelector(".filter").addEventListener("input", event => {
      this.filter = event.target.value;
      window.innerWidth <= 800 && event.target.scrollIntoView(true);
      m.redraw();
    });
  }

  view(vnode) {
    let venues = [...new Set(tournamentData.schedule.map(o => o.venue))].sort();

    let schedule = filterArray(this.filter, tournamentData.schedule, o => o.event);

    let eventDates = [];
    for (let o of schedule) {
      if (eventDates.length == 0 || o.date != eventDates[eventDates.length - 1][0]) {
        eventDates.push([o.date, []]);
      }

      let eventTimes = eventDates[eventDates.length - 1][1];
      if (eventTimes.length == 0 || o.time != eventTimes[eventTimes.length - 1][0]) {
        eventTimes.push([o.time, {}]);
      }

      let eventVenues = eventTimes[eventTimes.length - 1][1];
      if (!(o.venue in eventVenues)) {
        eventVenues[o.venue] = [o];
      } else {
        eventVenues[o.venue].push(o);
      }
    }

    return m("div",
      {
        id: vnode.attrs.id,
      },
      [
        m("input.filter", { placeholder: "Filter events" }),
        eventDates.length > 0 ? eventDates.map(d => m(
          "div.day",
          { key: d[0] },
          [
            m("h4.date", printDate(d[0], true)),
            m("div.events", { style: { "grid-template-columns": `8em repeat(${venues.length}, 1fr)` } }, [
              m("h5.header", { key: d[0] + "Time" }, "Time"),
              ...venues.map(v => m("h5.header", { key: d[0] + v }, v)),
              ...d[1].map(e => [
                m("div.time", { key: d[0] + e[0] }, e[0]),
                ...venues.map(v => m("div.venue-events", { key: d[0] + e[0] + v }, [
                  m("div.venue", v),
                  e[1][v] ? m("ul", e[1][v].map(event => m(
                    "li.event",
                    { key: event.event },
                    event.link ? m("a", { href: `/tournament/${tournamentId}/event/${encodeURIComponent(event.link)}` }, event.event) : event.event,
                  ))) : [],
                ])),
              ]).flat(),
            ]),
          ],
        )) : [m("p", { key: "no-matches" }, "No scheduled events match the filter")],
      ],
    );
  }
}

class Events {
  constructor() {
    this.filter = "";
  }

  oncreate(vnode) {
    vnode.dom.querySelector(".filter").addEventListener("input", event => {
      this.filter = event.target.value;
      window.innerWidth <= 800 && event.target.scrollIntoView(true);
      m.redraw();
    });
  }

  view(vnode) {
    let eventGroups = tournamentData.eventGroups.map(g => ({
      name: g.name,
      events: filterArray(this.filter, g.events, e => e.name).map(e => e.name).sort(),
    })).filter(g => g.events.length > 0);

    return m("div",
      {
        id: vnode.attrs.id,
      },
      [
        m("input.filter", { placeholder: "Filter events" }),
        eventGroups.length > 0 ? eventGroups.map(g => m(
          "div.event-group",
          { key: g.name },
          [
            m("h4.name", g.name),
            m("ul", g.events.map(e => m(
              "li.event",
              { key: e },
              m("a", { href: `/tournament/${tournamentId}/event/${encodeURIComponent(e)}` }, e),
            ))),
          ],
        )) : [m("p", { key: "no-matches" }, "No events match the filter")],
      ],
    );
  }
}


class Players {
  constructor() {
    this.filter = "";
  }

  oncreate(vnode) {
    vnode.dom.querySelector(".filter").addEventListener("input", event => {
      this.filter = event.target.value;
      window.innerWidth <= 800 && event.target.scrollIntoView(true);
      m.redraw();
    });
  }

  view(vnode) {
    let players = filterArray(this.filter, tournamentData.players, p => `${p.firstName} ${p.nickNames.join(" ")} ${p.lastName}`);

    return m("div",
      {
        id: vnode.attrs.id,
      },
      [
        m("input.filter", { placeholder: "Filter players" }),
        players.length > 0 ? [
          m(
            "h4.player-count",
            players.length < tournamentData.players.length ?
              `Showing ${players.length} out of ${tournamentData.players.length} players` :
              `${players.length} players`,
          ),
          m(
            "div.player-list",
            [
              m("h5.header", "Name"),
              m("h5.header", "From"),
              players.map(p => m.fragment(
                { key: p.id },
                [
                  m(
                    "a.player-name",
                    { href: `/tournament/${tournamentId}/player/${p.id}` },
                    m.trust(`${p.lastName}, ${p.firstName}${p.nickNames.length > 0 ? ' "' + p.nickNames.join('" "') + '"' : ""}`),
                  ),
                  m("p.player-from", m.trust(p.from)),
                ],
              )),
            ],
          ),
        ] : [m("p", "No players match the filter")],
      ],
    );
  }
}

let main = document.querySelector("main");

m.mount(main, {
  view: function () {
    return error ? m(Error) : (captcha !== null ? m(Captcha) : (tournamentData !== null ? m(Main) : m(Loading)));
  },
});