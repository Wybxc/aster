#import "@preview/exemel:0.1.0": to-xml
#import "/lib.typ": all-tags, page-count, post-url, posts-with-tag, published-posts, settings

#let paths = {
  let found = ("", "about/", "posts/", "tags/", "archives/")

  let posts = published-posts()
  let post-pages = page-count(posts)
  if post-pages > 1 {
    for page in range(2, post-pages + 1) {
      found.push("posts/" + str(page) + "/")
    }
  }
  for item in posts {
    found.push(post-url(item.entry.id).trim("/") + "/")
  }
  for tag in all-tags() {
    let base = "tags/" + tag.slug + "/"
    found.push(base)
    let tag-pages = page-count(posts-with-tag(tag.slug))
    if tag-pages > 1 {
      for page in range(2, tag-pages + 1) {
        found.push(base + str(page) + "/")
      }
    }
  }

  found.sorted()
}

#let sitemap = (
  tag: "urlset",
  attrs: (xmlns: "http://www.sitemaps.org/schemas/sitemap/0.9"),
  children: paths.map(path => (
    tag: "url",
    children: ((tag: "loc", children: (settings.site.url + path,)),),
  )),
)

#metadata(to-xml(sitemap, pretty: true)) <aster-endpoint>
