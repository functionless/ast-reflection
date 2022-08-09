const { javascript } = require("projen");
const project = new javascript.NodeProject({
  name: "@functionless/ast-reflection",
  entrypoint: "ast_reflection.wasm",
  defaultReleaseBranch: "main",
  jest: false,
  release: true,
  releaseToNpm: true,
  gitignore: ["/dist/", "/target/", "/ast_reflection.wasm"],
  peerDeps: ["@swc/core@1.2.218"],
  workflowBootstrapSteps: [
    {
      name: "Install rust",
      uses: "actions-rs/toolchain@v1",
      with: {
        toolchain: "stable",
        components: "rustfmt",
        profile: "minimal",
        override: true,
      },
    },
    {
      name: "Install wasm32-wasi",
      run: "rustup target add wasm32-wasi",
    },
  ],
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
