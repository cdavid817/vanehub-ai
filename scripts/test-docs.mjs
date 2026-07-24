import { resolve } from "node:path";
import { repositoryRoot, run, verifyMdbook } from "./docs-tooling.mjs";

verifyMdbook();

for (const book of [
  "docs/developer-guide",
  "docs/user-guide/en",
  "docs/user-guide/zh-CN",
]) {
  run("mdbook", ["test", resolve(repositoryRoot, book)]);
}

console.log("All mdBook navigation and supported Rust code samples passed.");
