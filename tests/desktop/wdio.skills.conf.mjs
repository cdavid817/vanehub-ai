import { createDesktopConfig } from "./wdio-shared.mjs";

export const config = await createDesktopConfig({
  specDirectory: "specs-skills",
});
