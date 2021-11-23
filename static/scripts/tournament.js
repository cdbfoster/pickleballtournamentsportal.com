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
        ...(tournamentData.listing.logoUrl !== null ? [m("img", { src: tournamentData.listing.logoUrl })] : []),
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
        ...(tournamentData.schedule.length != 0 ? [
          m(Accordion, {
            //expanded: true,
            title: m("h3", "Schedule"),
            content: [
              m(Schedule, { id: "schedule" })
            ],
          }),
        ] : []),
        ...(tournamentData.eventGroups.length != 0 ? [
          m(Accordion, {
            title: m("h3", "Events"),
            content: m("p", "Blah"),
          }),
        ] : []),
        ...(tournamentData.players.length != 0 ? [
          m(Accordion, {
            title: m("h3", "Players"),
            content: m("p", "Blah"),
          }),
        ] : []),
      ]),
    ];
  }
}

function filterArray(filter, array, key) {
  function sanitize(string) {
    let stripSymbols = /[!@#$%^&*()\[\]{}\/?'\"\\;:|<>_=]/g;
    return string.toLowerCase().replaceAll(stripSymbols, " ").split(/\s+/).filter(s => s.length > 0);
  }

  function matches(a, b) {
    return a.every(x => b.some(y => y.startsWith(x)));
  }

  filter = sanitize(filter);
  if (filter.length == 0) {
    return array;
  }

  return array.filter(o => matches(filter, sanitize(key(o))));
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
        ...(eventDates.length > 0 ? eventDates.map(d => m(
          "div.day",
          [
            m("h4.date", printDate(d[0], true)),
            m("div.events", { style: { "grid-template-columns": `8em repeat(${venues.length}, 1fr)` } }, [
              m("h5.header", "Time"),
              ...venues.map(v => m("h5.header", v)),
              ...d[1].map(e => [
                m("div.time", e[0]),
                ...venues.map(v => m("div.venue-events", [
                  m("div.venue", v),
                  m("ul", (e[1][v] || []).map(event => m(
                    "li.event",
                    event.link ? m("a", { href: `/tournament/${tournamentId}/event/${encodeURIComponent(event.event)}` }, event.event) : event.event,
                  ))),
                ])),
              ]),
            ]),
          ],
        )) : [m("p", "No scheduled events match the filter")]),
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