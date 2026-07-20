# Final Content Export

This directory contains the final local export used to remove the site's
Contentful dependency.

- `preview/` contains all non-archived entries and assets visible through the
  Contentful Preview API, including unpublished drafts.
- `delivery/` contains the published snapshot that was visible in production.
- `asset-manifest.json` maps every exported asset to its checked-in path under
  `site/public/content/assets/`.
- `manifest.json` records export counts and the four draft asset records that
  did not yet have uploaded files.

The deployed management token had already been revoked, so editor-interface
configuration and Contentful-archived/deleted records were not available.
All current preview and production content, content models, locale metadata,
and every available asset binary are preserved here.

The static site does not read these archive files at build time. Its editable
content is generated in `site/src/content/` and `site/src/content-data/`.
