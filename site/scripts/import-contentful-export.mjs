// @ts-nocheck
import { copyFile, mkdir, readFile, stat, writeFile } from "node:fs/promises"
import { basename, extname, join, relative } from "node:path"
import { fileURLToPath } from "node:url"

const LOCALE = "en-US"
const SITE_DIRECTORY = fileURLToPath(new URL("..", import.meta.url))
const REPOSITORY_ROOT = fileURLToPath(new URL("../..", import.meta.url))

const [previewDirectory, deliveryDirectory] = process.argv.slice(2)

if (!previewDirectory || !deliveryDirectory) {
  throw new Error(
    "Usage: node site/scripts/import-contentful-export.mjs <preview-export> <delivery-export>",
  )
}

const archiveDirectory = join(
  SITE_DIRECTORY,
  "content",
  "archive",
  "contentful",
)
const assetDirectory = join(SITE_DIRECTORY, "public", "content", "assets")
const dataDirectory = join(SITE_DIRECTORY, "src", "content-data")
const blogDirectory = join(SITE_DIRECTORY, "src", "content", "blog")
const legalDirectory = join(SITE_DIRECTORY, "src", "content", "legal")

for (
  const directory of [
    archiveDirectory,
    assetDirectory,
    dataDirectory,
    blogDirectory,
    legalDirectory,
  ]
) {
  await mkdir(directory, { recursive: true })
}

async function readExport(directory, name) {
  return JSON.parse(await readFile(join(directory, `${name}.json`), "utf8"))
}

const preview = {
  assets: await readExport(previewDirectory, "assets"),
  contentTypes: await readExport(previewDirectory, "content_types"),
  entries: await readExport(previewDirectory, "entries"),
  locales: await readExport(previewDirectory, "locales"),
}

const delivery = {
  assets: await readExport(deliveryDirectory, "assets"),
  contentTypes: await readExport(deliveryDirectory, "content_types"),
  entries: await readExport(deliveryDirectory, "entries"),
  locales: await readExport(deliveryDirectory, "locales"),
}

for (
  const [mode, directory] of [
    ["preview", previewDirectory],
    ["delivery", deliveryDirectory],
  ]
) {
  const target = join(archiveDirectory, mode)
  await mkdir(target, { recursive: true })
  for (const name of ["assets", "content_types", "entries", "locales"]) {
    await copyFile(
      join(directory, `${name}.json`),
      join(target, `${name}.json`),
    )
  }
}

const publishedEntryIds = new Set(
  delivery.entries.items.map((item) => item.sys.id),
)
const publishedAssetIds = new Set(
  delivery.assets.items.map((item) => item.sys.id),
)
const entriesById = new Map(
  preview.entries.items.map((item) => [item.sys.id, item]),
)
const assetsById = new Map(
  preview.assets.items.map((item) => [item.sys.id, item]),
)

function field(item, name) {
  return item.fields[name]?.[LOCALE]
}

function entryType(item) {
  return item.sys.contentType.sys.id
}

function linkId(value) {
  return value?.sys?.id ?? null
}

function safeFilename(asset) {
  const file = field(asset, "file")
  const original = file?.fileName || basename(file?.url || "") || asset.sys.id
  const extension = extname(original).toLowerCase()
  const stem = basename(original, extension)
    .normalize("NFKD")
    .replaceAll(/[^\x00-\x7F]/g, "")
    .replaceAll(/[^A-Za-z0-9._-]+/g, "-")
    .replaceAll(/^-+|-+$/g, "")
    .slice(0, 120)
  return `${stem || "asset"}${extension}`
}

function localAsset(asset) {
  if (!asset) {
    return null
  }

  const file = field(asset, "file")
  if (!file) {
    return {
      id: asset.sys.id,
      path: null,
      title: correctImportedContent(field(asset, "title") || ""),
      description: correctImportedContent(field(asset, "description") || ""),
      published: publishedAssetIds.has(asset.sys.id),
      sourceUrl: null,
    }
  }

  const filename = safeFilename(asset)
  return {
    id: asset.sys.id,
    path: `/content/assets/${asset.sys.id}/${filename}`,
    title: correctImportedContent(field(asset, "title") || ""),
    description: correctImportedContent(field(asset, "description") || ""),
    filename,
    contentType: file.contentType,
    size: file.details?.size ?? null,
    width: file.details?.image?.width ?? null,
    height: file.details?.image?.height ?? null,
    published: publishedAssetIds.has(asset.sys.id),
    sourceUrl: file.url ? `https:${file.url}` : null,
  }
}

const assetManifest = preview.assets.items
  .map(localAsset)
  .sort((left, right) => left.id.localeCompare(right.id))

await writeJson(join(archiveDirectory, "asset-manifest.json"), assetManifest)

let completedAssets = 0
const downloadableAssets = assetManifest.filter(
  (asset) => asset.sourceUrl && asset.path,
)

await mapConcurrent(downloadableAssets, 8, async (asset) => {
  const target = join(SITE_DIRECTORY, "public", asset.path)
  await mkdir(join(target, ".."), { recursive: true })

  let existingSize = null
  try {
    existingSize = (await stat(target)).size
  } catch {
    // The file has not been downloaded yet.
  }

  if (existingSize !== asset.size) {
    const response = await fetch(asset.sourceUrl)
    if (!response.ok) {
      throw new Error(
        `Failed to download asset ${asset.id}: ${response.status} ${response.statusText}`,
      )
    }
    const bytes = new Uint8Array(await response.arrayBuffer())
    if (asset.size !== null && bytes.byteLength !== asset.size) {
      throw new Error(
        `Asset ${asset.id} size mismatch: expected ${asset.size}, received ${bytes.byteLength}`,
      )
    }
    await writeFile(target, bytes)
  }

  completedAssets += 1
  if (
    completedAssets % 25 === 0
    || completedAssets === downloadableAssets.length
  ) {
    console.log(`assets: ${completedAssets}/${downloadableAssets.length}`)
  }
})

const assetById = new Map(assetManifest.map((asset) => [asset.id, asset]))

function assetFromLink(value) {
  const asset = assetById.get(linkId(value))
  if (!asset) {
    return null
  }
  const { sourceUrl: _sourceUrl, ...local } = asset
  return local
}

function entryFromLink(value) {
  return entriesById.get(linkId(value)) ?? null
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("\"", "&quot;")
}

function correctImportedContent(value) {
  return String(value)
    .replaceAll(/Incentiv.es/g, "Incentives")
    .replaceAll(/re-exc.te/g, "re-execute")
}

function renderText(node, { accentBold = false } = {}) {
  let html = escapeHtml(correctImportedContent(node.value || "")).replaceAll(
    "\n",
    "<br>",
  )
  for (const mark of node.marks || []) {
    switch (mark.type) {
      case "bold":
        html = accentBold
          ? `<span class="text-accent-500">${html}</span>`
          : `<strong>${html}</strong>`
        break
      case "italic":
        html = `<em>${html}</em>`
        break
      case "underline":
        html = `<u>${html}</u>`
        break
      case "strikethrough":
        html = `<s>${html}</s>`
        break
      case "code":
        html = `<code>${html}</code>`
        break
    }
  }
  return html
}

function renderChildren(node, options) {
  return (node.content || [])
    .map((child) => renderNode(child, options))
    .join("")
}

function renderNode(node, options = {}) {
  switch (node.nodeType) {
    case "document":
      return (node.content || [])
        .map((child) => renderNode(child, options))
        .join("\n")
    case "text":
      return renderText(node, options)
    case "paragraph": {
      const className = options.terms ? " class=\"mb-4 text-justify\"" : ""
      return `<p${className}>${renderChildren(node, options)}</p>`
    }
    case "heading-1": {
      const className = options.terms ? " class=\"text-xl mt-4 mb-4\"" : ""
      return `<h1${className}>${renderChildren(node, options)}</h1>`
    }
    case "heading-2": {
      const className = options.terms
        ? " class=\"text-lg mt-3 mb-3 font-bold\""
        : ""
      return `<h2${className}>${renderChildren(node, options)}</h2>`
    }
    case "heading-3":
      return `<h3>${renderChildren(node, options)}</h3>`
    case "heading-4":
      return `<h4>${renderChildren(node, options)}</h4>`
    case "heading-5":
      return `<h5>${renderChildren(node, options)}</h5>`
    case "heading-6":
      return `<h6>${renderChildren(node, options)}</h6>`
    case "unordered-list":
      return `<ul>${renderChildren(node, options)}</ul>`
    case "ordered-list":
      return `<ol>${renderChildren(node, options)}</ol>`
    case "list-item":
      return `<li>${renderChildren(node, options)}</li>`
    case "blockquote":
      return `<blockquote>${renderChildren(node, options)}</blockquote>`
    case "hr":
      return "<hr>"
    case "hyperlink":
      return `<a href="${escapeHtml(node.data?.uri || "")}">${
        renderChildren(
          node,
          options,
        )
      }</a>`
    case "embedded-asset-block": {
      const asset = assetById.get(node.data?.target?.sys?.id)
      if (!asset?.path) {
        return ""
      }
      const alt = asset.description || asset.title || asset.filename
      return `<img src="${asset.path}" alt="${escapeHtml(alt)}">`
    }
    default:
      return renderChildren(node, options)
  }
}

function renderTitle(document) {
  return (document?.content || [])
    .map((block) => renderChildren(block, { accentBold: true }))
    .join("<br>")
}

function pageRichText(document) {
  return renderNode(document, { accentBold: true })
}

function sourceMetadata(item) {
  return {
    sourceId: item.sys.id,
    createdAt: item.sys.createdAt,
    updatedAt: item.sys.updatedAt,
    firstPublishedAt: item.sys.firstPublishedAt ?? null,
    publishedAt: item.sys.publishedAt ?? null,
    published: publishedEntryIds.has(item.sys.id),
  }
}

const landingEntry = preview.entries.items.find(
  (item) => entryType(item) === "landing",
)
const learnEntry = preview.entries.items.find(
  (item) => entryType(item) === "learn",
)

if (!landingEntry || !learnEntry) {
  throw new Error(
    "The Contentful export is missing the landing or learn entry",
  )
}

const landing = {
  ...sourceMetadata(landingEntry),
  entry: field(landingEntry, "entry") || "",
}
for (const prefix of ["first", "second", "third", "fourth"]) {
  landing[`${prefix}TitleHtml`] = renderTitle(
    field(landingEntry, `${prefix}Title`),
  )
  landing[`${prefix}TextHtml`] = pageRichText(
    field(landingEntry, `${prefix}Text`),
  )
}
await writeJson(join(dataDirectory, "landing.json"), landing)

const learn = {
  ...sourceMetadata(learnEntry),
  entryTitle: field(learnEntry, "entryTitle") || "",
}
for (
  const prefix of [
    "cover",
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "sixth",
    "seventh",
    "eighth",
    "ninth",
  ]
) {
  learn[`${prefix}TitleHtml`] = renderTitle(
    field(learnEntry, `${prefix}Title`),
  )
  learn[`${prefix}TextHtml`] = pageRichText(field(learnEntry, `${prefix}Text`))
}
await writeJson(join(dataDirectory, "learn.json"), learn)

const ecosystem = preview.entries.items
  .filter((item) => entryType(item) === "ecosystem")
  .map((item) => ({
    ...sourceMetadata(item),
    name: field(item, "name") || "",
    url: field(item, "url") || "",
    logo: assetFromLink(field(item, "logo")),
    background: assetFromLink(field(item, "background")),
    categories: (field(item, "category") || [])
      .map((link) => {
        const category = entryFromLink(link)
        return category
          ? {
            id: category.sys.id,
            category: field(category, "category") || "",
            textColor: field(category, "textColor") || "#000000",
            bgColor: field(category, "bgColor") || "#ffffff",
          }
          : null
      })
      .filter(Boolean),
  }))
  .sort((left, right) => left.name.localeCompare(right.name))
await writeJson(join(dataDirectory, "ecosystem.json"), ecosystem)

const team = preview.entries.items
  .filter((item) => entryType(item) === "team")
  .map((item) => ({
    ...sourceMetadata(item),
    name: field(item, "name") || "",
    title: field(item, "title") || "",
    twitterHandle: field(item, "twitterHandle") || "",
    position: field(item, "position") ?? null,
    profilePicture: assetFromLink(field(item, "profilePicture")),
    twitterPicture: assetFromLink(field(item, "twitterPicture")),
  }))
  .sort((left, right) => {
    const leftPosition = left.position ?? Number.POSITIVE_INFINITY
    const rightPosition = right.position ?? Number.POSITIVE_INFINITY
    return leftPosition - rightPosition || left.name.localeCompare(right.name)
  })
await writeJson(join(dataDirectory, "team.json"), team)

const roadmap = {
  sections: preview.entries.items
    .filter((item) => entryType(item) === "roadmapSection")
    .map((item) => ({
      ...sourceMetadata(item),
      id: item.sys.id,
      section: field(item, "section") || "",
      position: field(item, "position") ?? null,
      description: field(item, "description") || "",
      percentComplete: field(item, "percentComplete") ?? null,
      slug: field(item, "slug") || "",
    })),
  subsections: preview.entries.items
    .filter((item) => entryType(item) === "roadmapSubsection")
    .map((item) => ({
      ...sourceMetadata(item),
      id: item.sys.id,
      subsection: field(item, "subsection") || "",
      linkedSectionId: linkId(field(item, "linkedSection")),
      position: field(item, "position") ?? null,
      description: field(item, "description") || "",
      percentComplete: field(item, "percentComplete") ?? null,
      slug: field(item, "slug") || "",
    })),
  milestones: preview.entries.items
    .filter((item) => entryType(item) === "roadmapMilestone")
    .map((item) => ({
      ...sourceMetadata(item),
      id: item.sys.id,
      milestone: field(item, "milestone") || "",
      linkedSubsectionId: linkId(field(item, "linkedSubsection")),
      position: field(item, "position") ?? null,
      description: field(item, "description") || "",
      markComplete: field(item, "markComplete") === true,
      incompleteIcon: assetFromLink(field(item, "incompleteIcon")),
      completeIcon: assetFromLink(field(item, "completeIcon")),
      slug: field(item, "slug") || "",
    })),
}
await writeJson(join(dataDirectory, "roadmap.json"), roadmap)

const termsSlugs = new Map([
  ["5ZQLR5qjQqRRr62puERKbQ", "privacy-policy"],
  ["7r42B6KGZ79je8cTEEyvom", "terms-of-service"],
  ["43sR1iKG6dHDFNXrVlKIF3", "airdrop-terms-and-conditions"],
  ["4BgHT0VqZ0kPyf4HVQ1Jhw", "auro-privacy-policy"],
  ["3mXKRCugy1uUhBEjLucDWp", "auro-terms-of-service"],
])

for (
  const item of preview.entries.items.filter(
    (item) => entryType(item) === "terms",
  )
) {
  const slug = termsSlugs.get(item.sys.id) || item.sys.id
  const metadata = sourceMetadata(item)
  const frontmatter = [
    "---",
    `title: ${yamlString(field(item, "title") || "")}`,
    `sourceId: ${yamlString(metadata.sourceId)}`,
    `published: ${metadata.published}`,
    `createdAt: ${yamlString(metadata.createdAt)}`,
    `updatedAt: ${yamlString(metadata.updatedAt)}`,
    "---",
    "",
  ].join("\n")
  const body = renderNode(field(item, "copy"), { terms: true })
  await writeFile(
    join(legalDirectory, `${slug}.md`),
    `${frontmatter}${body}\n`,
  )
}

for (
  const item of preview.entries.items.filter(
    (item) => entryType(item) === "blog",
  )
) {
  const slug = field(item, "slug")
  if (!slug) {
    throw new Error(`Blog entry ${item.sys.id} has no slug`)
  }

  const metadata = sourceMetadata(item)
  const cover = assetFromLink(field(item, "cover"))
  const frontmatter = [
    "---",
    `title: ${yamlString(field(item, "title") || "")}`,
    `date: ${yamlString(field(item, "date") || "")}`,
    `author: ${yamlString(field(item, "author") || "union_build")}`,
    `description: ${yamlString(field(item, "description") || "")}`,
    `cover: ${yamlString(cover?.path || "")}`,
    `coverAlt: ${yamlString(cover?.description || cover?.title || "")}`,
    `hidden: ${field(item, "hidden") === true}`,
    `published: ${metadata.published}`,
    `sourceId: ${yamlString(metadata.sourceId)}`,
    `createdAt: ${yamlString(metadata.createdAt)}`,
    `updatedAt: ${yamlString(metadata.updatedAt)}`,
    "---",
    "",
  ].join("\n")
  const body = renderNode(field(item, "content"))
  await writeFile(join(blogDirectory, `${slug}.md`), `${frontmatter}${body}\n`)
}

await writeJson(join(archiveDirectory, "manifest.json"), {
  generatedAt: new Date().toISOString(),
  locale: LOCALE,
  preview: {
    contentTypes: preview.contentTypes.total,
    entries: preview.entries.total,
    assets: preview.assets.total,
    locales: preview.locales.total,
  },
  delivery: {
    contentTypes: delivery.contentTypes.total,
    entries: delivery.entries.total,
    assets: delivery.assets.total,
    locales: delivery.locales.total,
  },
  downloadedAssets: downloadableAssets.length,
  missingAssetFiles: assetManifest
    .filter((asset) => !asset.path)
    .map((asset) => asset.id),
})

console.log(
  `generated ${relative(REPOSITORY_ROOT, blogDirectory)} (${
    preview.entries.items.filter((item) => entryType(item) === "blog").length
  } posts)`,
)
console.log(
  `generated ${relative(REPOSITORY_ROOT, legalDirectory)} (${
    preview.entries.items.filter((item) => entryType(item) === "terms").length
  } documents)`,
)

function yamlString(value) {
  return JSON.stringify(String(value))
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`)
}

async function mapConcurrent(items, concurrency, operation) {
  let index = 0
  async function worker() {
    while (index < items.length) {
      const current = items[index]
      index += 1
      await operation(current)
    }
  }
  await Promise.all(Array.from({ length: concurrency }, () => worker()))
}
