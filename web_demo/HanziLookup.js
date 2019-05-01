class HanziLookup {
  constructor(worker_uri, wasm_uri) {
    this.worker = new Worker(worker_uri);
    this.worker.onmessage = this.handleMessage.bind(this);
    this.worker.postMessage({ "init": wasm_uri });
  }

  barf(data) {
    return new Promise((resolve, reject) => {
      let mwhaha = `barf_finished(${data})`;
      document.addEventListener(`barf_finished(${data})`, (event) => {
        resolve(event.detail.result);
      });
      this.worker.postMessage({ data: data });
    });
  }

  handleMessage(event) {
    document.dispatchEvent(new CustomEvent(`barf_finished(${event.data.source})`, {
      detail: {
        result: event.data.result,
      }
    }));
  }
}
