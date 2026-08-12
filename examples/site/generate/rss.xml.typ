#import "@preview/exemel:0.1.0": to-xml
#import "/lib.typ": get-collection, settings

#let rss-date(value) = {
  let (year, month, day) = value.split("-").map(int)
  datetime(year: year, month: month, day: day).display(
    "[weekday repr:short], [day padding:zero] [month repr:short] [year] 00:00:00 GMT",
  )
}

#let entries = {
  get-collection("journal")
    .map(entry => (entry: entry, metadata: entry.metadata()))
    .sorted(key: item => item.metadata.date)
    .rev()
}

#let items = entries.map(item => {
  let metadata = item.metadata
  let path = "/journal/" + item.entry.id + "/"
  let page = sys.inputs._aster.site.pages.find(page => page.path == path)
  let url = settings.site.url + path.trim("/") + "/"
  (
    tag: "item",
    children: (
      (tag: "title", children: (metadata.title,)),
      (tag: "link", children: (url,)),
      (tag: "guid", attrs: ("isPermaLink": "true"), children: (url,)),
      (tag: "pubDate", children: (rss-date(metadata.date),)),
      (tag: "description", children: (metadata.summary,)),
      (tag: "content:encoded", children: (page.content.html,)),
    ),
  )
})

#let feed = (
  tag: "rss",
  attrs: (version: "2.0", "xmlns:content": "http://purl.org/rss/1.0/modules/content/"),
  children: (
    (
      tag: "channel",
      children: (
        (tag: "title", children: (settings.site.title,)),
        (tag: "link", children: (settings.site.url,)),
        (tag: "description", children: (settings.site.description,)),
        (tag: "language", children: (settings.site.language,)),
        (tag: "lastBuildDate", children: (rss-date(entries.first().metadata.date),)),
        ..items,
      ),
    ),
  ),
)

#metadata(to-xml(feed, pretty: true)) <aster-output>
