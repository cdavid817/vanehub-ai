// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../i18n";
import { DirectorySettingField } from "./directory-setting-field";

describe("DirectorySettingField", () => {
  it("commits on Enter, shows a saved hint, and does not re-save an unchanged value on blur", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn<(value: string) => Promise<void>>().mockResolvedValue(undefined);
    render(
      <DirectorySettingField ariaLabel="目录" canBrowse={false} disabled={false} onPick={async () => null} onSave={onSave} placeholder="" value="/old" />,
    );

    const input = screen.getByRole("textbox", { name: "目录" });
    await user.clear(input);
    await user.type(input, "/new{Enter}");
    expect(onSave).toHaveBeenCalledWith("/new");
    expect(await screen.findByText("已保存")).toBeTruthy();

    await user.tab();
    expect(onSave).toHaveBeenCalledTimes(1);
  });

  it("treats a stored extended-length path as unchanged when it blurs in display form", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn<(value: string) => Promise<void>>().mockResolvedValue(undefined);
    render(
      <DirectorySettingField ariaLabel="目录" canBrowse={false} disabled={false} onPick={async () => null} onSave={onSave} placeholder="" value={"\\\\?\\D:\\Code"} />,
    );
    const input = screen.getByRole("textbox", { name: "目录" }) as HTMLInputElement;
    expect(input.value).toBe("D:\\Code");
    await user.click(input);
    await user.tab();
    expect(onSave).not.toHaveBeenCalled();
  });

  it("commits the picked directory and surfaces a save failure inline", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn<(value: string) => Promise<void>>().mockRejectedValue(new Error("not a directory"));
    render(
      <DirectorySettingField ariaLabel="目录" canBrowse disabled={false} onPick={async () => "/picked"} onSave={onSave} placeholder="" value="" />,
    );
    await user.click(screen.getByRole("button", { name: "浏览…" }));
    await waitFor(() => expect(onSave).toHaveBeenCalledWith("/picked"));
    expect(await screen.findByText("not a directory")).toBeTruthy();
  });
});
