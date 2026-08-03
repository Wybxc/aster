#import "@preview/exemel:0.1.0": to-xml
#import "/lib/aster/content.typ": get-collection

#let rss-date(value) = {
  let (year, month, day) = value.split("-").map(int)
  datetime(year: year, month: month, day: day).display(
    "[weekday repr:short], [day padding:zero] [month repr:short] [year] 00:00:00 GMT",
  )
}

#let entries = {
  get-collection("journal")
    .map(entry => (entry: entry, rendered: entry.render()))
    .sorted(key: item => item.rendered.metadata.date)
    .rev()
}

#let items = entries.map(item => {
  let metadata = item.rendered.metadata
  let url = sys.inputs.site.url + "journal/" + item.entry.id + "/"
  (
    tag: "item",
    children: (
      (tag: "title", children: (metadata.title,)),
      (tag: "link", children: (url,)),
      (tag: "guid", attrs: ("isPermaLink": "true"), children: (url,)),
      (tag: "pubDate", children: (rss-date(metadata.date),)),
      (tag: "description", children: (metadata.summary,)),
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
        (tag: "title", children: (sys.inputs.site.title,)),
        (tag: "link", children: (sys.inputs.site.url,)),
        (tag: "description", children: (sys.inputs.site.description,)),
        (tag: "language", children: (sys.inputs.site.language,)),
        (tag: "lastBuildDate", children: (rss-date(entries.first().rendered.metadata.date),)),
        ..items,
      ),
    ),
  ),
)

#metadata(to-xml(feed, pretty: true)) <endpoint>
