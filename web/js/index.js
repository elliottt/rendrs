// vim: et sw=2 ts=2


class OutputManager {
  constructor() {
    this.known_outputs = [];
  }

  hasOutput(name) {
    for (const output of this.known_outputs) {
      if (output.name == name) {
        return output.node;
      }
    }

    return null;
  }

  addOutput(name, node) {
    this.known_outputs.push({ name: name, node: node });
  }
};

const mgr = new OutputManager();

const con = new WebSocket(`ws://${window.location.host}/ws`);

con.onmessage = event => {
  const message = JSON.parse(event.data);
  document.title = message.scene;

  const outputs = document.getElementById('outputs');

  // TODO: keep outputs sorted according to the order in the message from the
  // server
  // TODO: remove outputs that don't occur in the message from the server
  message.outputs.forEach(output => {
    const node = mgr.hasOutput(output.name);
    if (node != null) {
      console.log(`updating ${output.name}`);
      updateOutput(node, output);
    } else {
      console.log(`creating ${output.name}`);
      const node = makeOutput(output);
      mgr.addOutput(output.name, node);
      outputs.appendChild(node);
    }
  });
};

function makeOutput(output) {
  const container = document.createElement('div');
  container.classList.add('container');

  const title = document.createElement('div');
  title.innerText = output.name;
  title.classList.add('title');
  container.appendChild(title);

  switch (output.type) {
    case "ascii":
      container.classList.add('ascii');
      const pre = document.createElement('pre');
      pre.innerText = output.content;
      container.appendChild(pre);
      break;

    case "file":
      container.classList.add('image');
      const image = document.createElement('img');
      image.src = `/output/${output.name}?t=${Date.now()}`;
      container.appendChild(image);
      break;
  }

  return container;
}

function updateOutput(node, output) {
  switch (output.type) {
    case "ascii":
      const pre = node.getElementsByTagName('pre')[0];
      pre.innerText = output.content;
      break;

    case "file":
      const image = node.getElementsByTagName('img')[0];
      image.src = `/output/${output.name}?t=${Date.now()}`;
      break;
  }
}
