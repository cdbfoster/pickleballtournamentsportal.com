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

    return m("ul.team", { class: vnode.attrs.class }, team.map(p => m("li.player", m(PlayerName, { player: p, link: vnode.attrs.link }))));
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

      let bracket = document.querySelector("#bracket") || document.querySelector("#round-robin");
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
  return node !== null ? (node.hasOwnProperty("match") ? node.match.winner : (node.hasOwnProperty("seed") ? node.seed : node)) : [];
}

function teamIdTag(team) {
  return team !== null ? nodePlayers(team).map(p => p.id).sort().join("-") : null;
}

function nodePlayersMatch(a, b) {
  return a !== null && b !== null && teamIdTag(a) === teamIdTag(b);
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

class RoundRobinBracket {
  constructor() {
    this.filter = "";
  }

  oncreate(vnode) {
    vnode.dom.addEventListener("filter", event => {
      this.filter = event.detail;
      m.redraw();
    })
  }

  view() {
    return m("div#round-robin", [
      m(RoundRobinStandings, { filter: this.filter }),
      m("div.rounds", eventData.bracket.roundRobin.map((r, i) => m(RoundRobinRound, { key: i, title: `Round ${i + 1}`, round: r, filter: this.filter }))),
    ]);
  }
}

class RoundRobinStandings {
  view(vnode) {
    let filter = vnode.attrs.filter;

    let teamIds = eventData.teams.map(t => teamIdTag(t));
    let teamData = {};
    for (let t of teamIds) {
      teamData[t] = {
        wins: 0,
        headToHead: {},
        pointDifferential: 0,
        points: 0,
      };

      teamIds.forEach(u => teamData[t].headToHead[u] = 0);
    };

    for (let round of eventData.bracket.roundRobin) {
      for (let match of round) {
        if (match.winner.length > 0) {
          let winner = teamIdTag(match.children[roundRobinIsMatchWinner(match.children[0], match) ? 0 : 1]);
          let d = match.scores.reduce((t, s) => t += s[0] - s[1], 0);

          match.children.forEach((c, i, a) => {
            let childTag = teamIdTag(c);
            if (childTag === winner) {
              teamData[childTag].wins += 1;
              teamData[childTag].headToHead[teamIdTag(a[(i + 1) % 2])] = 1;
              teamData[childTag].pointDifferential += d;
              teamData[childTag].points += match.scores.map(s => s[0]).reduce((s, p) => s + p, 0);
            } else {
              teamData[childTag].pointDifferential -= d;
              teamData[childTag].points += match.scores.map(s => s[1]).reduce((s, p) => s + p, 0);
            }
          });
        }
      }
    }

    let standings = sortArray(eventData.teams, t => {
      let id = teamIdTag(t);

      let h2hWins = teamIds
        .filter(u => teamData[id].wins == teamData[u].wins)
        .map(u => teamData[id].headToHead[u])
        .reduce((s, w) => s + w, 0);

      return [
        teamData[id].wins,
        h2hWins,
        teamData[id].pointDifferential,
        teamData[id].points,
      ].join("-");
    }).reverse();

    return m("div.standings", [
      m("h4.header.ordinal"),
      m("h4.header", "Team"),
      m("h4.header", "Wins"),
      m("h4.header", "PD"),
      standings.map(t => {
        let id = teamIdTag(t);
        let data = teamData[id];
        let filtered = filter.length > 0 ? !nodeMatchesFilter(t, filter) : false;
        return m.fragment({
          key: id,
        }, [
          m("div.ordinal", { class: filtered ? "filtered" : undefined }),
          m(Team, { team: t, link: true, class: filtered ? "filtered" : undefined }),
          m("div.wins", { class: filtered ? "filtered" : undefined }, data.wins),
          m("div.pd", { class: filtered ? "filtered" : undefined }, data.pointDifferential),
        ]);
      }),
    ]);
  }
}

class RoundRobinRound {
  view(vnode) {
    let title = vnode.attrs.title;
    let round = vnode.attrs.round;
    let filter = vnode.attrs.filter;
    let bye = eventData.teams
      .find(t => round
        .map(n => n.children
          .map(c => c.seed.map(p => p.id)))
          .flat(2)
          .every(i => !t.map(p => p.id).includes(i))
      );

    return m("div.round", [
      m("h3.title", title),
      m("ul.matches", round
        .map((n, i) => m("li", m(RoundRobinMatch, { key: i, match: n })))
        .filter((n, i) => filter.length == 0 || round[i].children.some(c => nodeMatchesFilter(c, filter)))),
      bye && filter.length == 0 || nodeMatchesFilter(bye, filter) ? m("div.bye", [
        m("p", "bye"),
        m(Team, { team: bye, link: false }),
      ]) : [],
    ]);
  }
}

function roundRobinIsMatchWinner(child, match) {
  return match.winner.length > 0 && teamIdTag(match.winner) === teamIdTag(child);
}

class RoundRobinMatch {
  view(vnode) {
    let match = vnode.attrs.match;

    return m("div.match", [
      match.children.map(c => m(Team, {
        class: match.winner.length > 0 ? (roundRobinIsMatchWinner(c, match) ? "winner" : "loser") : undefined,
        team: c.seed,
        link: false,
      })),
      m("p.vs", "vs"),
      m("ul.scores", match.scores.map(s => m("li", s.join("-")))),
    ]);
  }
}

let main = document.querySelector("main");

m.mount(main, {
  view: function () {
    return error ? m(Error) : (captcha !== null ? m(Captcha) : (eventData !== null ? m(Main) : m(Loading)));
  },
});
