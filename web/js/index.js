
const con = new WebSocket(`ws://${window.location.host}/ws`);

con.onmessage = event => {
  const message = JSON.parse(event.data);
  clearOutputs();
  message.outputs.forEach(appendOutput);
};

function clearOutputs() {
  const outputs = document.getElementById('outputs');
  outputs.innerHTML = '';
}

function appendOutput(output) {
  const container = document.createElement('div');
  container.classList.add('container');

  switch (output.type) {
    case "ascii":
      container.classList.add('ascii');
      const pre = document.createElement('pre');
      pre.innerText = output.content;
      container.appendChild(pre);
      break;

    case "file":
      console.log(output);
      container.classList.add('image');
      const image = document.createElement('img');
      image.src = `/output/${output.name}?t=${Date.now()}`;
      container.appendChild(image);
      break;
  }

  document.getElementById('outputs').appendChild(container);
}
