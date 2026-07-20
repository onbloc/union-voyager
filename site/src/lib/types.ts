import type { ImageMetadata } from "astro"

export type MaybePromise<T> = T | Promise<T>

export type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P]
}

export type NamedImage = {
  name: string
  url: string
  logo: ImageMetadata
}

export type LocalAsset = {
  id: string
  path: string | null
  title: string
  description: string
  filename?: string
  contentType?: string
  size?: number | null
  width?: number | null
  height?: number | null
  published: boolean
}
