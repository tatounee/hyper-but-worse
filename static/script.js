let evtSource = null;

let eventList = document.getElementById("event-list");

function startStream() {
  console.log("evtSource", evtSource === null);
  if (evtSource === null) {
    evtSource = new EventSource("/events");

    evtSource.onmessage = (event) => {
      const msgNode = document.createElement("li");
      msgNode.innerHTML = `<pre>${JSON.stringify(event)}</pre>`;

      eventList.appendChild(msgNode);
    };
  }
}
