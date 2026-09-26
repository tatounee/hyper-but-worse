let evtSource = null;

let eventList = document.getElementById("event-list");

function startStream() {
  console.log("evtSource", evtSource === null);
  if (evtSource === null) {
    evtSource = new EventSource("/sse/counter");

    evtSource.onmessage = (event) => {
      console.log("MESSAGE", event);
      const msgNode = document.createElement("li");

      msgNode.innerHTML = `<pre>${JSON.stringify({ type: event.type, data: event.data, id: event.lastEventId })}</pre>`;

      eventList.appendChild(msgNode);
    };
  }
}

function stopStream() {
  if (evtSource === null) return;

  evtSource.close();
  evtSource = null;
}
