import { defineCollection, z } from "astro:content"

const blogCollection = defineCollection({
  type: "content",
  schema: z.object({
    title: z.string(),
    date: z.coerce.date(),
    author: z.string().default("union_build"),
    description: z.string().default(""),
    cover: z.string(),
    coverAlt: z.string().default(""),
    hidden: z.boolean().default(false),
    published: z.boolean().default(true),
    sourceId: z.string(),
    createdAt: z.coerce.date(),
    updatedAt: z.coerce.date(),
  }),
})

const legalCollection = defineCollection({
  type: "content",
  schema: z.object({
    title: z.string(),
    published: z.boolean().default(true),
    sourceId: z.string(),
    createdAt: z.coerce.date(),
    updatedAt: z.coerce.date(),
  }),
})

export const collections = {
  blog: blogCollection,
  legal: legalCollection,
}
