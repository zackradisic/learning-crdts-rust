import * as Proto from "./proto";

export const createRuntime = async (): Promise<Proto.Exports> => {
  const bytes = await fetch("/ligma.wasm").then((res) => res.arrayBuffer());
  const runtime: Proto.Exports = await Proto.createRuntime(bytes, {
    log: (str) => console.log(str),
  });
  return runtime;
};
