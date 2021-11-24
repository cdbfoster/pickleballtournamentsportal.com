var tournamentData = null;
var captcha = null;

fetch(`/tournament/${tournamentId}/data`)
  .then(response => response.json())
  .then(data => {
    if ("tournamentData" in data) {
      tournamentData = data.tournamentData;
    } else if ("captcha" in data) {
      captcha = data.captcha;
    } else {
      console.error("Error: " + data.error.reason);
    }
    m.redraw();
  })
  .catch(error => console.error(error));

class Main {
  view(vnode) {

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
            title: m("h3", "Schedule"),
            content: m(Schedule, { id: "schedule" }),
          }),
        ] : [],
        tournamentData.eventGroups.length != 0 ? [
          m(Accordion, {
            title: m("h3", "Events"),
            content: m(Events, { id: "events" }),
          }),
        ] : [],
        tournamentData.players.length != 0 ? [
          m(Accordion, {
            title: m("h3", "Players"),
            content: m("p", "Blah"),
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
                  m("ul", (e[1][v] || []).map(event => m(
                    "li.event",
                    { key: event.event },
                    event.link ? m("a", { href: `/tournament/${tournamentId}/event/${encodeURIComponent(event.link)}` }, event.event) : event.event,
                  ))),
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

let main = document.querySelector("main");

m.mount(main, {
  view: function () {
    return captcha !== null ? m(Captcha) : (tournamentData !== null ? m(Main) : m(Loading));
  },
});