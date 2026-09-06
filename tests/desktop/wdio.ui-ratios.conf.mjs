import { createDesktopConfig } from "./wdio-shared.mjs";

// One spec that resizes the real window through several aspect ratios and audits every session
// workspace tab and the Basic Configuration page for layout defects at each size.
export const config = await createDesktopConfig({
  specDirectory: "specs-ui-ratios",
});
