const { javascript } = require("projen");
const project = new javascript.NodeProject({
  name: "@functionless/ast-reflection",
  main: "ast_reflection.wasm",
  defaultReleaseBranch: "main",
  jest: false,
  release: true,
  releaseToNpm: true,
  gitignore: ["/dist/", "/target/"],
});

project.addPackageIgnore("/.gitattributes");
project.addPackageIgnore("/target/");
project.addPackageIgnore("/tests/");
project.addPackageIgnore("/src/");
project.addPackageIgnore("/.github/");
project.addPackageIgnore("/Cargo.lock");
project.addPackageIgnore("/Cargo.toml");
project.addPackageIgnore("/rust-toolchain");
project.addPackageIgnore("/rustfmt.toml");
project.addPackageIgnore("/dist/");

project.compileTask.exec(
  "cargo build --release --target wasm32-wasi && cp target/wasm32-wasi/release/ast_reflection.wasm ."
);

project.synth();
