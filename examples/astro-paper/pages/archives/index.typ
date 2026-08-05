#import "/lib.typ": date-value, published-posts
#import "/components/page-header.typ": page-header
#import "/templates/site.typ": site

#let posts = published-posts()
#let years = {
  let grouped = (:)
  for item in posts {
    let year = item.metadata.date.slice(0, 4)
    grouped.insert(year, grouped.at(year, default: ()) + (item,))
  }
  grouped
}

#show: site.with(
  title: "Archives",
  description: "A chronological archive of every AsterPaper article.",
  path: "/archives/",
  active: "archives",
)

#metadata("./index.css") <aster-style>
#html.main(id: "main-content")[
  #page-header([Archives], [All published articles grouped by year.])
  #for year in years.keys().sorted().rev() {
    let entries = years.at(year).map(item => list.item([
      #html.time(datetime: date-value(item.metadata.date))[
        #item.metadata.date.slice(5, 10)
      ] #link("/posts/" + item.entry.id + "/")[#item.metadata.title]
    ]))
    html.section[
      #heading(level: 1)[#year]
      #list(..entries)
    ]
  }
]
