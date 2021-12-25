var eventData = null;
var captcha = null;
var error = false;

fetch(`/tournament/${tournamentId}/event/${eventName}/data`)
  .then(response => response.json())
  .then(data => {
    if ("eventData" in data) {
      eventData = data.eventData;
      document.title = `${eventData.name} | ${eventData.tournament.name} | PickleballTournamentsPortal.com`;

      for (t in eventData.teams) {
        eventData.teams[t] = sortArray(eventData.teams[t], p => p.lastName + p.firstName);
      }
      eventData.teams = sortArray(eventData.teams, t => t.map(p => p.lastName));
    } else if ("captcha" in data) {
      captcha = data.captcha;
    } else {
      console.error("Error: " + data.error.reason);

      if (data.error.reason == "tournament not found" || data.error.reason == "event not found") {
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
    return [
      m(EventInfo),
      m(TeamList),
      eventData.bracket ? [
        eventData.bracket.hasOwnProperty("doubleElim") ? m(DoubleElimBracket) : m(RoundRobinBracket),
      ] : [],
    ]
  }
}

class EventInfo {
  view() {
    return m("div#event-info", [
      m("a.tournament-name", { href: `/tournament/${eventData.tournament.id}` }, m.trust(eventData.tournament.name)),
      m("h2.event-name", eventData.name),
      eventData.scheduleItem ? [
        m("p.event-date", printDate(eventData.scheduleItem.date, true)),
        m("p.event-time", eventData.scheduleItem.time),
        m("p.event-venue", eventData.scheduleItem.venue),
      ] : [],
    ]);
  }
}

class PlayerName {
  view(vnode) {
    let player = vnode.attrs.player;

    let first = player.nickNames.length == 0 ? player.firstName : player.nickNames[player.nickNames.length - 1];
    let last = player.lastName;

    return m(
      (vnode.attrs.link ? "a" : "p") + ".player-name",
      {
        href: `/tournament/${eventData.tournament.id}/player/${player.id}`,
      },
      `${first} ${last}`,
    )
  }
}

class Team {
  view(vnode) {
    let team = vnode.attrs.team;

    return m("ul.team", team.map(p => m("li.player", m(PlayerName, { player: p, link: vnode.attrs.link }))));
  }
}

class TeamList {
  constructor() {
    this.filter = "";
  }

  oncreate(vnode) {
    vnode.dom.querySelector(".filter").addEventListener("input", event => {
      this.filter = event.target.value;
      window.innerWidth <= 800 && event.target.scrollIntoView(true);

      let bracket = document.querySelector("#bracket");
      if (bracket !== null) {
        bracket.dispatchEvent(new CustomEvent("filter", {
          detail: this.filter,
        }));
      }

      m.redraw();
    });
  }

  view() {
    let teams = filterArray(
      this.filter,
      eventData.teams,
      t => t.map(
        p => `${p.nickNames.length > 0 ? p.nickNames[p.nickNames.length - 1] : p.firstName} ${p.lastName}`
      ).join(" "),
    );

    return m("div#team-list", [
      m("input.filter", { placeholder: "Filter teams" }),
      teams.length > 0 ? m("ul", teams.map(t => m(
        "li",
        { key: t.map(p => p.id).join(",") },
        m(Team, { team: t, link: true }),
      ))) : m("p", "No teams match the filter"),
    ]);
  }
}

function nodePlayers(node) {
  return node !== null ? (node.hasOwnProperty("match") ? node.match.winner : node.seed) : [];
}

function nodePlayersMatch(a, b) {
  if (a === null || b === null) {
    return false;
  }

  let ai = nodePlayers(a).map(p => p.id);
  let bi = nodePlayers(b).map(p => p.id);

  return ai.length > 0 && ai.length == bi.length && ai.every((ap, i) => ap == bi[i]);
}

function nodeMatchesFilter(node, filter) {
  let players = nodePlayers(node);
  return players.some(p => matchesFilter(
    `${p.nickNames.length > 0 ? p.nickNames[p.nickNames.length - 1] : p.firstName} ${p.lastName}`,
    filter,
  ));
}

function nodeIsSelected(node, selectedNode, filter) {
  return nodePlayersMatch(node, selectedNode) || nodeMatchesFilter(node, filter);
}

function filterBracket(roots, filter) {
  function getMatchingRoots(parentMatches, node, filter) {
    if (node.hasOwnProperty("seed")) {
      return [];
    }

    let nodeMatches = nodeMatchesFilter(node, filter);
    let roots = [];

    node.match.children.forEach(c => roots = roots.concat(getMatchingRoots(nodeMatches, c, filter)));

    if (!parentMatches && !nodeMatches) {
      if (node.match.children.some(c => nodeMatchesFilter(c, filter))) {
        roots.push(node);
      }
    }

    return roots;
  }

  let rootNodes = roots.map(r => { return { match: r }; });
  let newRoots = rootNodes.filter(r => nodeMatchesFilter(r, filter));
  rootNodes.forEach(r => newRoots = newRoots.concat(getMatchingRoots(nodeMatchesFilter(r, filter), r, filter)));

  function copyNode(node) {
    return node.hasOwnProperty("match") ? { match: {...node.match} } : { seed: node.seed };
  }

  function pruneNode(node, filter) {
    if (node.hasOwnProperty("seed")) {
      return node;
    }

    node.match.children = node.match.children.map(c => {
      if (!nodeMatchesFilter(c, filter)) {
        return { seed: nodePlayers(c) };
      } else {
        return pruneNode(copyNode(c), filter);
      }
    });

    return node;
  }

  newRoots = newRoots.map(r => pruneNode(copyNode(r), filter));

  return newRoots.map(r => r.match);
}

class DoubleElimBracket {
  constructor() {
    this.drag = false;
    this.selected = null;
    this.filter = "";
  }

  oncreate(vnode) {
    vnode.dom.addEventListener("mousemove", event => {
      if (event.buttons == 1) {
        vnode.dom.scrollLeft -= event.movementX;
        window.scrollBy(0, -event.movementY);
      }
    });

    vnode.dom.addEventListener("bracketselect", event => {
      if (event.detail.hold) {
        if (this.selected !== null && this.selected.hold && nodePlayersMatch(this.selected, event.detail)) {
          this.selected = null;
        } else {
          this.selected = event.detail;
        }
        m.redraw();
      } else if (this.selected === null || !this.selected.hold) {
        this.selected = event.detail;
        m.redraw();
      }
    });

    vnode.dom.addEventListener("bracketdeselect", event => {
      if (this.selected !== null && !this.selected.hold) {
        this.selected = null;
        m.redraw();
      }
    });

    vnode.dom.addEventListener("filter", event => {
      this.filter = event.detail;
      m.redraw();
    });
  }

  view() {
    let bracket = this.filter.length > 0 ?
      eventData.bracket.doubleElim.map(b => [b[0], filterBracket([b[1]], this.filter)]) :
      eventData.bracket.doubleElim.map(b => [b[0], [b[1]]]);

    return m(
      "div#bracket",
      bracket
        .filter(b => b[1].length > 0)
        .map(b => [
          ...(b[0] !== null ? [m("h4", { key: b[0] }, b[0])] : []),
          ...b[1].map(c => m(BracketMatch, {
            key: "match" + c.id,
            match: c,
            selected: nodeIsSelected({ match: c }, this.selected, this.filter),
            selectedNode: this.selected,
            filter: this.filter,
          })),
        ]),
    );
  }
}

class BracketMatch {
  oncreate(vnode) {
    vnode.dom.querySelector(".results").addEventListener("click", e => {
      if (!e.target.matches("a")) {
        e.target.dispatchEvent(
          new CustomEvent("bracketselect", {
            bubbles: true,
            detail: {
              match: vnode.attrs.match,
              hold: true,
            },
          }),
        );
      }
    });

    vnode.dom.querySelector(".results").addEventListener("mouseenter", e => e.target.dispatchEvent(
      new CustomEvent("bracketselect", {
        bubbles: true,
        detail: {
          match: vnode.attrs.match,
          hold: false,
        },
      }),
    ));

    vnode.dom.querySelector(".results").addEventListener("mouseleave", e => e.target.dispatchEvent(
      new CustomEvent("bracketdeselect", {
        bubbles: true,
        detail: {
          match: vnode.attrs.match,
        },
      }),
    ));
  }

  view(vnode) {
    let match = vnode.attrs.match;
    let selected = vnode.attrs.selected;
    let selectedNode = vnode.attrs.selectedNode;
    let filter = vnode.attrs.filter;
    let linkTo = match.loserTo || match.winnerTo;

    return m("div.match", {
      class: selected ? "selected" : (selectedNode || filter.length != "" ? "unselected" : undefined),
    }, [
      m(
        "div.results",
        {
          id: `match-${match.id}`,
        },
        [
          m("p.match-id", `#${match.id}`),
          m(Team, { team: match.winner, link: false }),
          m("ul.scores", match.scores.map(s => m("li", s.join("-")))),
          linkTo ? m(
            "p",
            {
              class: match.loserTo ? "loser-to" : "winner-to",
            }, [
              `${match.loserTo ? "Loser" : "Winner"} to `,
              m("a", {
                href: `#match-${linkTo}`,
                onclick: event => {
                  let target = document.querySelector(`#match-${linkTo}`);
                  let c = "highlight";
                  let t1 = 500;
                  let t2 = 4000;

                  // Due to filtering, the target might not exist, so blink the origin
                  if (target === null) {
                    target = event.target.parentElement.parentElement;
                    c = "not-present";
                    t1 = 100;
                    t2 = 400;
                  }

                  target.classList.add(`${c}-base`);
                  target.classList.add(c);
                  setTimeout(() => target.classList.remove(c), t1);
                  setTimeout(() => target.classList.remove(`${c}-base`), t2);
                },
              }, `#${linkTo}`),
            ],
          ) : [],
        ],
      ),
      m("ul.children", match.children.map(b => m(
        "li",
        {
          key: b.hasOwnProperty("match") ? "match" + b.match.id : "seed" + b.seed.map(p => p.id).join(","),
          class: nodePlayers({ match: match }).length > 0 && nodePlayersMatch({ match: match }, b) ? "winner" : "loser",
        },
        b.hasOwnProperty("match") ?
          m(BracketMatch, {
            match: b.match,
            selected: nodeIsSelected(b, selectedNode, filter),
            selectedNode: selectedNode,
            filter: filter,
          }) :
          m(BracketSeed, {
            seed: b.seed,
            selected: nodeIsSelected(b, selectedNode, filter),
            selectedNode: selectedNode,
            filter: filter,
          }),
      ))),
    ]);
  }
}

class BracketSeed {
  oncreate(vnode) {
    vnode.dom.addEventListener("click", () => vnode.dom.dispatchEvent(
      new CustomEvent("bracketselect", {
        bubbles: true,
        detail: {
          seed: vnode.attrs.seed,
          hold: true,
        },
      }),
    ));

    vnode.dom.addEventListener("mouseenter", () => vnode.dom.dispatchEvent(
      new CustomEvent("bracketselect", {
        bubbles: true,
        detail: {
          seed: vnode.attrs.seed,
          hold: false,
        },
      }),
    ));

    vnode.dom.addEventListener("mouseleave", () => vnode.dom.dispatchEvent(
      new CustomEvent("bracketdeselect", {
        bubbles: true,
        detail: {
          seed: vnode.attrs.seed,
        },
      }),
    ));
  }

  view(vnode) {
    let seed = vnode.attrs.seed;
    let selected = vnode.attrs.selected;
    let selectedNode = vnode.attrs.selectedNode;
    let filter = vnode.attrs.filter;

    return m("div.seed", {
      class: selected ? "selected" : (selectedNode || filter.length != "" ? "unselected" : undefined),
    }, m(Team, { team: seed, link: false }));
  }
}

let main = document.querySelector("main");

m.mount(main, {
  view: function () {
    return error ? m(Error) : (captcha !== null ? m(Captcha) : (eventData !== null ? m(Main) : m(Loading)));
  },
});
