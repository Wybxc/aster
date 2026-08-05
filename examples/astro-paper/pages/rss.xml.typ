#import "@preview/exemel:0.1.0": to-xml
#import "/lib.typ": date-value, post-url, published-posts, settings

#let rss-date(value) = date-value(value).display(
  "[weekday repr:short], [day padding:zero] [month repr:short] [year] 00:00:00 GMT",
)

#let posts = published-posts()
#let latest-date = if posts.len() > 0 {
  posts.first().metadata.date
} else {
  datetime.today().display("[year]-[month padding:zero]-[day padding:zero]")
}
#let items = posts.map(item => {
  let metadata = item.metadata
  let url = settings.site.url + post-url(item.entry.id).trim("/") + "/"
  (
    tag: "item",
    children: (
      (tag: "title", children: (metadata.title,)),
      (tag: "link", children: (url,)),
      (tag: "guid", attrs: ("isPermaLink": "true"), children: (url,)),
      (tag: "pubDate", children: (rss-date(metadata.date),)),
      (tag: "description", children: (metadata.description,)),
    ),
  )
})

#let feed = (
  tag: "rss",
  attrs: (version: "2.0"),
  children: (
    (
      tag: "channel",
      children: (
        (tag: "title", children: (settings.site.title,)),
        (tag: "link", children: (settings.site.url,)),
        (tag: "description", children: (settings.site.description,)),
        (tag: "language", children: (settings.site.language,)),
        (tag: "lastBuildDate", children: (rss-date(latest-date),)),
        ..items,
      ),
    ),
  ),
)

#metadata(to-xml(feed, pretty: true)) <aster-endpoint>
