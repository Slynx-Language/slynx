# Releasing Slynx

Repository releases are tag-driven and validated through the Node/TypeScript workflow.

## Release Checklist

1. Update `CHANGELOG.md` when needed.
2. Confirm CI is green on `main`.
3. Run:

```bash
npm install
npm run check
npm test
npm run build
```

4. Create and push an annotated `v*` tag.
5. Let GitHub Actions publish the GitHub Release.

## Tag Format

- `v0.1.0`
- `v0.1.1`
- `v0.2.0-rc.1`

## GitHub Release Behavior

Pushing a `v*` tag triggers `.github/workflows/release.yml`, which validates the TypeScript workspace and then publishes a GitHub Release with generated notes.
