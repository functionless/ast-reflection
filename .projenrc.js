const { javascript } = require("projen");
const project = new javascript.NodeProject({
  defaultReleaseBranch: "main",
  name: "ast-reflection",

  // deps: [],                /* Runtime dependencies of this module. */
  // description: undefined,  /* The description is just a string that helps people understand the purpose of the package. */
  // devDeps: [],             /* Build dependencies for this module. */
  // packageName: undefined,  /* The "name" in package.json. */
});
project.synth();