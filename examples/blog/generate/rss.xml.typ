#import "@preview/exemel:0.1.0": to-xml
#import "/lib.typ": date-value, post-url, posts, settings

#let rss-date(value) = date-value(value).display(
  "[weekday repr:short], [day padding:zero] [month repr:short] [year] 00:00:00 GMT",
)

#let items = posts().map(item => {
  let data = item.metadata
  let path = post-url(item.entry.id)
  let page = sys.inputs._aster.site.pages.find(page => page.path == path)
  let url = settings.site.url + path.trim("/") + "/"
  (
    tag: "item",
    children: (
      (tag: "title", children: (data.title,)),
      (tag: "link", children: (url,)),
      (tag: "guid", attrs: ("isPermaLink": "true"), children: (url,)),
      (tag: "pubDate", children: (rss-date(data.date),)),
      (tag: "description", children: (data.description,)),
      (tag: "content:encoded", children: (page.content.html,)),
    ),
  )
})

#let feed = (
  tag: "rss",
  attrs: (version: "2.0", "xmlns:content": "http://purl.org/rss/1.0/modules/content/"),
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

#metadata(to-xml(feed, pretty: true)) <aster-output>
