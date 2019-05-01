class HanziLookupWebWorker {
  constructor(wasm_uri) {
    importScripts("hanzi_lookup.js");
    this.instance = wasm_bindgen(wasm_uri);
  }

  barf(data) {
    return this.instance.then(() => wasm_bindgen.barf(data));
  }
}

let memoized_worker;

onmessage = (event) => {
  if ("init" in event.data) {
    memoized_worker = new HanziLookupWebWorker(event.data["init"]);
  } else if ("data" in event.data) {
    memoized_worker.barf(event.data.data).then(output => {
      postMessage({ source: event.data.data, result: output });
    });
  }
};
