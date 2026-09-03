import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../i18n";
import { LoopVerificationCommandEditor } from "./loop-verification-command-editor";

describe("LoopVerificationCommandEditor", () => {
  it("renders repeatable structured command controls with localized errors", () => {
    const html = renderToStaticMarkup(<LoopVerificationCommandEditor commands={[
      { id: "tests", program: "", arguments: "run\ntest", workingDirectory: "packages/app", timeoutSeconds: 120, required: true },
      { id: "lint", program: "npm", arguments: "run\nlint", workingDirectory: "", timeoutSeconds: 60, required: false },
    ]} onChange={() => undefined} showErrors />);

    expect(html).toContain("命令 1");
    expect(html).toContain("添加命令");
    expect(html).toContain("相对工作目录");
    expect(html).toContain("参数（每行一项）");
    expect(html).toContain("必需检查");
    expect(html).toContain("每条验证命令都必须填写程序");
    expect(html).toContain('title="下移命令"');
    expect(html).toContain('title="删除命令"');
    // Task 17.5: this error now renders through the shared `FieldError` primitive instead of a
    // hand-rolled paragraph -- check for markup only that component produces (its own icon plus
    // `role="alert"` wrapped in its specific layout classes) so a revert to a bespoke lookalike
    // `<p>` would be caught, not just a check that the message text is present somewhere.
    expect(html).toContain('role="alert"');
    expect(html).toContain('class="mt-1 flex items-start gap-1 text-xs text-destructive"');
  });
});
