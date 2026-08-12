#import "@preview/exemel:0.1.0": to-xml
#import "/lib.typ": date-value, post-url, published-posts, settings

#let atom-date(value) = if value.len() == 10 { value + "T00:00:00Z" } else { value }
#let updated-date(metadata) = if metadata.modified == none {
  metadata.date
} else {
  metadata.modified
}

#let posts = published-posts()
#let latest-date = if posts.len() > 0 {
  posts
    .map(item => updated-date(item.metadata))
    .sorted()
    .last()
} else {
  datetime.today().display("[year]-[month padding:zero]-[day padding:zero]T00:00:00Z")
}
#let entries = posts.map(item => {
  let metadata = item.metadata
  let updated = updated-date(metadata)
  let path = post-url(item.entry.id)
  let page = sys.inputs._aster.site.pages.find(page => page.path == path)
  let url = settings.site.url + path.trim("/") + "/"
  (
    tag: "entry",
    children: (
      (tag: "title", children: (metadata.title,)),
      (tag: "id", children: (url,)),
      (tag: "link", attrs: (href: url)),
      (tag: "published", children: (atom-date(metadata.date),)),
      (tag: "updated", children: (atom-date(updated),)),
      (tag: "summary", children: (metadata.description,)),
      (tag: "content", attrs: (type: "html", "xml:base": url), children: (page.content.html,)),
    ),
  )
})

#let feed = (
  tag: "feed",
  attrs: (xmlns: "http://www.w3.org/2005/Atom", "xml:lang": settings.site.language),
  children: (
    (tag: "title", children: (settings.site.title,)),
    (tag: "subtitle", children: (settings.site.description,)),
    (tag: "id", children: (settings.site.url,)),
    (tag: "link", attrs: (href: settings.site.url)),
    (tag: "link", attrs: (
      rel: "self",
      type: "application/atom+xml",
      href: settings.site.url + "atom.xml",
    )),
    (tag: "updated", children: (atom-date(latest-date),)),
    (tag: "author", children: ((tag: "name", children: (settings.site.author,)),)),
    ..entries,
  ),
)

#metadata(to-xml(feed, pretty: true)) <aster-output>
