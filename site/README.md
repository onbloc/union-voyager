# Union Site

[union.build](https://union.build) introduces Union and shows our [blog](https://union.build/blog).

## Quickstart

Run the following to start a development server, once it's running edit the files in `site/` and you'll see your changes reflected immediately in the browser.

```sh
nix run .#site-dev-server
```

## Architecture

It's a fully static [Astro] site. All copy, posts, structured page data, and
media assets are checked into this repository, so builds do not require a CMS
or network access. Styling is done using [Tailwind]. 3D models are made using
[Spline].

Local content lives in:

- `src/content/blog/` and `src/content/legal/` for authored documents.
- `src/content-data/` for landing, learning, ecosystem, team, and roadmap data.
- `public/content/assets/` for the locally hosted media library.
- `content/archive/contentful/` for the final source export and migration audit trail.

Run `pnpm --filter=site build` from the repository root to create the static
site in `site/dist/`.

[astro]: https://astro.build
[spline]: https://spline.design
[tailwind]: https://tailwindcss.com
