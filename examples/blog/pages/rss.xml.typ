#import "@preview/exemel:0.1.0": to-xml
#import "/lib.typ": date-value, post-url, posts, settings

#let rss-date(value) = date-value(value).display(
  "[weekday repr:short], [day padding:zero] [month repr:short] [year] 00:00:00 GMT",
)

#let items = posts().map(item => {
  let data = item.metadata
  let url = settings.site.url + post-url(item.entry.id).trim("/") + "/"
  (
    tag: "item",
    children: (
      (tag: "title", children: (data.title,)),
      (tag: "link", children: (url,)),
      (tag: "guid", attrs: ("isPermaLink": "true"), children: (url,)),
      (tag: "pubDate", children: (rss-date(data.date),)),
      (tag: "description", children: (data.description,)),
    ),
  )
})

#let feed = (
  tag: "rss",
  attrs: (version: "2.0"),
  children: ((
    tag: "channel",
    children: (
      (tag: "title", children: (settings.site.title,)),
      (tag: "link", children: (settings.site.url,)),
      (tag: "description", children: (settings.site.description,)),
      (tag: "language", children: (settings.site.language,)),
      ..items,
    ),
  ),),
)

#metadata(to-xml(feed, pretty: true)) <aster-endpoint>
