import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import path from "node:path";
import { waitForLiveNativeBridge } from "../helpers/feishu-live.mjs";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

globalThis.describe("VaneHub AI live Feishu credential cleanup", () => {
  globalThis.it("clears the run-owned credential after every live phase", async () => {
    const resultDir = globalThis.process.env.VANEHUB_DESKTOP_RESULT_DIR;
    assert.ok(resultDir, "live result directory disappeared before cleanup");
    await waitForLiveNativeBridge();
    let status = "CLEARED";
    try {
      await invoke(({ core }) => core.invoke("clear_im_connector", { kind: "feishu" }));
      const connectors = await invoke(({ core }) => core.invoke("list_im_connectors"));
      const feishu = connectors.find((connector) => connector.descriptor.kind === "feishu");
      if (!feishu || feishu.hasCredentials || feishu.config.credentialRef) status = "FAILED";
    } catch {
      status = "FAILED";
    }
    await writeFile(
      path.join(resultDir, "feishu-live-credential-cleanup.json"),
      `${JSON.stringify({ status, credentialProfileOwned: true }, null, 2)}\n`,
      "utf8",
    );
    assert.equal(status, "CLEARED", "the run-owned Feishu credential was not cleared");
  });
});
