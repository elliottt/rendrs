
const con = new WebSocket(`ws://${window.location.host}/ws`);

con.onmessage = event => {
  const message = JSON.parse(event.data);
  console.log(message);
};
