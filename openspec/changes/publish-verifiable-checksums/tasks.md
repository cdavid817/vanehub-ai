## 1. Repair the published release

- [x] 1.1 Rewrite the `v0.1.0-preview.1` manifest to name each asset as GitHub serves it, keeping the digests the build produced
- [x] 1.2 Prove the corrected manifest verifies a fully downloaded asset and that the original does not, in the same directory
- [x] 1.3 Re-upload it over the existing `SHA256SUMS` asset and confirm the served copy

## 2. Fix the publishing job

- [x] 2.1 Generate the manifest by basename with spaces mapped to dots, rather than piping runner paths through `xargs sha256sum`
- [x] 2.2 Fail the job when two entries would share a published name
- [x] 2.3 Echo the manifest into the job log so a regression is visible in the run
- [x] 2.4 Exercise the loop and the collision guard locally against fixture files with spaces in their names, confirming the guard fires rather than assuming it

## 3. Fix the guidance

- [x] 3.1 State in the release notes that the manifest names assets as served and belongs in the same directory as the download
- [x] 3.2 Note that a mismatch on a large asset is more often a truncated download than tampering, after two `gh release download` attempts returned different truncated sizes for one asset

## 4. Verification

- [x] 4.1 `npm run lint:ci`, `npm run test`, `npm run build`, `npm run docs:check`
- [x] 4.2 `openspec validate publish-verifiable-checksums --strict` and `openspec validate --specs --strict`
- [x] 4.3 Confirm the workflow YAML still parses and the publish job's step order is unchanged

## 5. Next release

- [ ] 5.1 On the next tag, download an asset and its manifest and run the documented command before announcing the release
