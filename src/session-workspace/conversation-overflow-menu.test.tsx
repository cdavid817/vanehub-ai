import { renderToStaticMarkup } from "react-dom/server";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../i18n";
import { ConversationOverflowMenu } from "./conversation-overflow-menu";

describe("conversation overflow menu", () => {
  beforeAll(async () => activateAppLanguage("zh-CN"));

  it("exposes an accessible expanded-state trigger", () => {
    const html = renderToStaticMarkup(
      <ConversationOverflowMenu
        infoPanelExpanded={false}
        onToggleInfoPanel={() => undefined}
        onToggleSessionList={() => undefined}
        onToggleWorkspaceTabs={() => undefined}
        sessionListExpanded
        workspaceTabsExpanded
      />,
    );
    expect(html).toContain('aria-haspopup="menu"');
    expect(html).toContain('aria-expanded="false"');
    expect(html).toContain('aria-label="会话选项"');
  });
});
