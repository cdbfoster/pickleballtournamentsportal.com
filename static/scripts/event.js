var eventData = null;
var captcha = null;
var error = false;

fetch(`/tournament/${tournamentId}/event/${eventName}/data`)
  .then(response => response.json())
  .then(data => {
    if ("eventData" in data) {
      eventData = data.eventData;
      document.title = `${eventData.name} | ${eventData.tournament.name} | PickleballTournamentsPortal.com`;
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