import rss from "@astrojs/rss"
import type { APIContext } from "astro"
import { type CollectionEntry, getCollection } from "astro:content"

type BlogEntry = CollectionEntry<"blog">

export async function GET(context: APIContext) {
  const blog = await getCollection("blog")
  const site = context.site
  if (!site) {
    throw new Error("Missing site metadata")
  }

  return rss({
    site,
    title: "The Union Blog",
    description:
      "Union is a hyper-efficient, zero-knowledge interoperability layer that connects Appchains, Layer 1, and Layer 2 networks.",
    items: blog
      .filter((post: BlogEntry) => post.data.published && !post.data.hidden)
      .sort(
        (left: BlogEntry, right: BlogEntry) => right.data.date.getTime() - left.data.date.getTime(),
      )
      .map((post: BlogEntry) => ({
        title: post.data.title,
        pubDate: post.data.date,
        link: `/blog/${post.slug}/`,
        description: post.data.description,
      })),
    // (optional) inject custom xml
    customData: `<language>en-us</language>`,
  })
}
