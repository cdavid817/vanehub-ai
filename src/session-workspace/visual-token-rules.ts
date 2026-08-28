/**
 * What "shared semantic tokens only" means, as something a test can check.
 *
 * Defined once and read by every guard that enforces it. Two copies of these patterns would drift,
 * and the copy that drifts is always the one with fewer readers — at which point one surface is
 * being held to a rule the other is not, and nothing says so.
 *
 * A scrim is deliberately outside the palette pattern. `bg-black/30` over a dialog is an absolute
 * on purpose, it is what the application's own dialog component uses, and it is the one place
 * where following the theme would be wrong. It carries no number, so the pattern does not reach
 * it.
 */

/** A colour taken from Tailwind's palette rather than from the theme. */
export const PALETTE_COLOR =
  /\b(?:bg|text|border|ring|fill|stroke|from|via|to|shadow|decoration|outline|accent|caret|divide|placeholder)-(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\d{2,3}\b/;

/** An arbitrary value that is a colour rather than a token. `hsl(var(--panel))` is the token form. */
export const LITERAL_COLOR = /\[(?:#[0-9a-fA-F]{3,8}|rgba?\(|hsla?\((?!var\()|oklch\()/;

/**
 * Strings the patterns must and must not match.
 *
 * Every guard asserts these before scanning anything. A typo in either pattern leaves a check that
 * passes because it matches nothing at all — the failure a source scan hides best, and the one
 * that makes a green guard worse than no guard.
 */
export const TOKEN_RULE_SAMPLES = {
  literalMatches: 'className="bg-[#0f172a]"',
  literalRejects: 'className="bg-[hsl(var(--panel))]"',
  paletteMatches: 'className="text-red-500"',
  paletteRejects: 'className="text-muted-foreground"',
} as const;
