## 1. Bounded Transcript Rendering

- [x] 1.1 Add a transcript row class that lets the browser skip off-screen rows while keeping them in the document, with a remembered intrinsic size so the scrollbar does not resize as rows enter and leave view
- [x] 1.2 Carry the class on the message row

## 2. Verification

- [x] 2.1 Run `npm run lint:ci`, `npm run test`, and `npm run build`
- [x] 2.2 Run `npm run docs:screenshots:check` — containment skips off-screen rendering, so full-page captures are the check that matters here
- [x] 2.3 Run `npm run architecture:check` and `openspec validate --specs --strict`
- [ ] 2.4 Confirm on a running client that scrolling back through a long transcript shows real content with no position shift

## Notes

Task 2.4 is a reader-facing behaviour that neither jsdom nor a byte-exact screenshot observes: both render the page once, and what is being checked is what happens across scrolling. The documentation screenshots do establish that a full-page capture still contains every row.

Virtualization was considered and deliberately not taken; the proposal records the reasoning.
