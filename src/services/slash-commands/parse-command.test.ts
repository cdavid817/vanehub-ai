import { describe, expect, it } from "vitest";
import { parseCommandInput } from "./parse-command";

describe("parseCommandInput", () => {
  it("treats ordinary prose as a message", () => {
    expect(parseCommandInput("hello world")).toEqual({ kind: "message" });
    expect(parseCommandInput("")).toEqual({ kind: "message" });
    expect(parseCommandInput("   ")).toEqual({ kind: "message" });
  });

  it("parses a bare command and lowercases the name", () => {
    expect(parseCommandInput("/help")).toEqual({ kind: "command", name: "help", args: [] });
    expect(parseCommandInput("  /Help  ")).toEqual({ kind: "command", name: "help", args: [] });
  });

  it("splits arguments on runs of whitespace", () => {
    expect(parseCommandInput("/mode   plan")).toEqual({ kind: "command", name: "mode", args: ["plan"] });
    expect(parseCommandInput("/export json extra")).toEqual({ kind: "command", name: "export", args: ["json", "extra"] });
  });

  it("unescapes a doubled slash into literal text", () => {
    expect(parseCommandInput("//help")).toEqual({ kind: "literal", content: "/help" });
    expect(parseCommandInput("//usr/bin/env python")).toEqual({ kind: "literal", content: "/usr/bin/env python" });
  });

  it("leaves paths and multi-line input alone", () => {
    expect(parseCommandInput("/usr/bin/env")).toEqual({ kind: "message" });
    expect(parseCommandInput("/help\nsecond line")).toEqual({ kind: "message" });
    expect(parseCommandInput("/1234")).toEqual({ kind: "message" });
    expect(parseCommandInput("/")).toEqual({ kind: "message" });
  });

  it("recognises the prefix a completion dropdown should react to", () => {
    expect(parseCommandInput("/mod")).toEqual({ kind: "command", name: "mod", args: [] });
  });
});
