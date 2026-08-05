#import "/lib.typ": published-posts
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
#html.elem("main", attrs: (id: "main-content"))[
  #page-header([Archives], [All published articles grouped by year.])
  #for year in years.keys().sorted().rev() {
    html.elem("section")[
      #html.elem("h2")[#year]
      #html.elem("ul")[
        #for item in years.at(year) {
          html.elem("li")[
            #html.elem("time", attrs: (datetime: item.metadata.date))[
              #item.metadata.date.slice(5, 10)
            ]
            #html.elem("a", attrs: (href: "/posts/" + item.entry.id + "/"))[
              #item.metadata.title
            ]
          ]
        }
      ]
    ]
  }
]
