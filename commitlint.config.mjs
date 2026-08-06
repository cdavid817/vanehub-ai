export default {
  extends: ["@commitlint/config-conventional"],
  rules: {
    // config-conventional defaults plus "deps": the repo's established
    // convention for dependency bumps — deps(npm)/deps(cargo)/deps(actions).
    "type-enum": [
      2,
      "always",
      [
        "build",
        "chore",
        "ci",
        "deps",
        "docs",
        "feat",
        "fix",
        "perf",
        "refactor",
        "revert",
        "style",
        "test",
      ],
    ],
  },
};
