const { javascript } = require("projen");
const project = new javascript.NodeProject({
  name: "@functionless/ast-reflection",
  main: "dist/ast_reflection.wasm",
  defaultReleaseBranch: "main",
  jest: false,
  release: true,
  releaseToNpm: true,
});

project.compileTask.exec(
  "rm -rf dist && mkdir dist && cargo build --release --target wasm32-wasi && cp target/wasm32-wasi/release/ast_reflection.wasm ./dist/"
);

project.synth();
