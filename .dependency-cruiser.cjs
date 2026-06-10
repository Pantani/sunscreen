const moduleRoot = "^\\.dependency-cruiser-rust/modules/src/";
const layerRoot = "^\\.dependency-cruiser-rust/layers/src/";

const modulePath = (pattern) => `${moduleRoot}${pattern}`;

const interfaceModules = "(?:cli|onboarding|tui)(?:/|\\.js$)";
const foundationalModules = "(?:(?:config|templates|rustpatch|fsutil|strings|process)(?:/|\\.js$)|error\\.js$)";
const applicationModules = "(?:cli|onboarding|bootstrap|runtime|plugin|codegen|scaffold|toolchain|workspace|tui)(?:/|\\.js$)";

/** @type {import("dependency-cruiser").IConfiguration} */
module.exports = {
  forbidden: [
    {
      name: "no-circular-rust-modules",
      severity: "error",
      comment: "Rust source modules must not form circular dependency chains.",
      from: { path: moduleRoot },
      to: { circular: true },
    },
    {
      name: "no-circular-rust-layers",
      severity: "error",
      comment: "Top-level Rust architecture modules/layers must not depend on each other cyclically.",
      from: { path: layerRoot },
      to: { circular: true },
    },
    {
      name: "lower-layers-must-not-import-interface",
      severity: "error",
      comment: "Only the interface layer may depend on cli/onboarding/tui modules.",
      from: {
        path: modulePath("(?!(?:main|lib)\\.js$)(?!(?:cli|onboarding|tui)(?:/|\\.js$)).+"),
      },
      to: {
        path: modulePath(interfaceModules),
      },
    },
    {
      name: "foundational-modules-stay-foundational",
      severity: "error",
      comment: "Shared foundation modules must not depend on application or interface modules.",
      from: {
        path: modulePath(foundationalModules),
      },
      to: {
        path: modulePath(applicationModules),
      },
    },
    {
      name: "codegen-must-not-import-runtime",
      severity: "error",
      comment: "Runtime may orchestrate codegen, but codegen must not depend on runtime.",
      from: {
        path: modulePath("codegen(?:/|\\.js$)"),
      },
      to: {
        path: modulePath("runtime(?:/|\\.js$)"),
      },
    },
    {
      name: "plugin-runtime-boundary",
      severity: "error",
      comment: "Plugin runtime must stay independent from CLI, runtime orchestration, generators, scaffolders, and template internals.",
      from: {
        path: modulePath("plugin(?:/|\\.js$)"),
      },
      to: {
        path: modulePath("(?:cli|onboarding|runtime|codegen|scaffold|templates|toolchain|tui|rustpatch|fsutil)(?:/|\\.js$)"),
      },
    },
    {
      name: "workspace-boundary",
      severity: "error",
      comment: "Workspace discovery/model code must stay below application capabilities and depend only on low-level shared modules.",
      from: {
        path: modulePath("workspace(?:/|\\.js$)"),
      },
      to: {
        path: modulePath("(?:cli|onboarding|runtime|plugin|codegen|scaffold|templates|toolchain|tui|rustpatch|fsutil|strings)(?:/|\\.js$)"),
      },
    },
    {
      name: "toolchain-runtime-boundary",
      severity: "error",
      comment: "Toolchain code may use the shared process boundary, but must not couple to runtime orchestration.",
      from: {
        path: modulePath("toolchain(?:/|\\.js$)"),
      },
      to: {
        path: modulePath("runtime(?:/|\\.js$)"),
      },
    },
  ],
  options: {
    doNotFollow: {
      path: "node_modules",
    },
    enhancedResolveOptions: {
      extensions: [".js"],
    },
    reporterOptions: {
      dot: {
        collapsePattern: "node_modules/[^/]+",
      },
    },
  },
};
