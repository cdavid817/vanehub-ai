# Usage statistics

Token usage across four dimensions — time, Agent, session, and model — and what those numbers do and do not count.

Scheduled tasks and notifications are in [Scheduled tasks and notifications](scheduled-tasks.md).

## Where to look

**Settings → Usage Statistics**, filterable by time range and Agent, showing a summary, trends, and a per-Agent breakdown.

## Four dimensions

Four cards at the top of the page:

| Card | Meaning |
| --- | --- |
| **Real total tokens** | The sum of input, output, and cache tokens the CLI reported |
| **Input tokens** | Input tokens excluding cache reads |
| **Output tokens** | Including reasoning output the CLI reported |
| **Cache tokens** | The sum of cache-read and cache-creation tokens |

The underlying data records **cache reads** and **cache writes** separately (both visible in the session info panel); the interface's "cache tokens" is their sum.

Three auxiliary metrics are also shown: **estimated total characters** (a substitute measure when there is no real token record, and not added to token counts), **real-data coverage** (the proportion of assistant responses with real CLI token counts), and **session count**.

**"Real-data coverage" is worth watching** — it tells you directly how many responses got real measurement, and when the proportion is low, so is the reference value of the trend chart.

![The Usage Statistics settings page showing token cards, daily trend, and per-Agent usage](assets/screenshots/usage-en.png)

## What the numbers mean

> **Usage data comes from each CLI's own reporting; VaneHub AI does not meter tokens independently.**

The page has a dedicated section explaining the measure, and you should read it before using this data for cost accounting.

The four CLIs each report usage in a different format, and the system handles each one separately. Collection is idempotent — it samples periodically while the terminal is open and once more on exit, and repeated collection does not double count.

## Notes and limits

- **Desktop only.**
- **OnePiece usage does not come through the terminal collection path**, so it is counted differently from the four CLIs.
