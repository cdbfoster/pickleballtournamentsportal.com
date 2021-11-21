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
    return [
      m("p", "Hello world!"),
    ];
  }
}

let main = document.querySelector("main");

m.mount(main, {
  view: function () {
    return captcha !== null ? m(Captcha) : (tournamentData !== null ? m(Main) : m(Loading));
  },
});