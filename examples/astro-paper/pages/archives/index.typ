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
)

#metadata(
  ```css
  main > section {
    display: grid;
    grid-template-columns: 5rem 1fr;
    border-top-width: 1px;
    padding-block: 1.25rem;
  }

  main > section h2 {
    font-size: 1.25rem;
    font-weight: 600;
    line-height: 1.75rem;
  }

  main > section li {
    display: grid;
    grid-template-columns: 4rem 1fr;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }

  main > section time {
    color: var(--muted-foreground);
    font-size: 0.875rem;
    line-height: 1.25rem;
  }

  @media (max-width: 639px) {
    main > section {
      grid-template-columns: minmax(0, 1fr);
      gap: 0.75rem;
    }
  }
  ```
) <aster-style>

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
